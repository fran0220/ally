use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lru::LruCache;
use once_cell::sync::Lazy;
use reqwest::header::USER_AGENT;
use tokio::{
    sync::Mutex,
    time::{self, MissedTickBehavior},
};
use tracing::{error, info};

use crate::{errors::AppError, media};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_CACHE_SIZE: usize = 100;
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
const DEFAULT_LOG_PREFIX: &str = "[image-cache]";
const IMAGE_DOWNLOADER_USER_AGENT: &str = "Mozilla/5.0 (compatible; ImageDownloader/1.0)";
const DOWNLOAD_LOG_PREFIX_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, Default)]
pub struct GetImageBase64Options<'a> {
    pub log_prefix: Option<&'a str>,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageCacheStats {
    pub cache_size: usize,
    pub valid_entries: usize,
    pub total_size_kb: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: u64,
    pub total_download_time_ms: u64,
}

#[derive(Clone)]
struct CacheEntry {
    data_url: Arc<String>,
    expires_at: Instant,
    size_bytes: usize,
}

type DownloadResult = Result<Arc<String>, AppError>;
type SharedDownloadFuture = Shared<BoxFuture<'static, DownloadResult>>;

static IMAGE_CACHE: Lazy<Mutex<LruCache<String, CacheEntry>>> = Lazy::new(|| {
    let capacity = NonZeroUsize::new(MAX_CACHE_SIZE).expect("MAX_CACHE_SIZE must be non-zero");
    Mutex::new(LruCache::new(capacity))
});
static IN_FLIGHT_DOWNLOADS: Lazy<Mutex<HashMap<String, SharedDownloadFuture>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static TOTAL_DOWNLOAD_TIME_MS: AtomicU64 = AtomicU64::new(0);
static CLEANUP_TASK_STARTED: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

pub async fn get_image_base64_cached(
    image_url: &str,
    options: GetImageBase64Options<'_>,
) -> Result<String, AppError> {
    ensure_cleanup_task_started();

    let trimmed = image_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_params("image url cannot be empty"));
    }
    if trimmed.starts_with("data:") {
        return Ok(trimmed.to_string());
    }

    let log_prefix = options.log_prefix.unwrap_or(DEFAULT_LOG_PREFIX);
    let cache_key = trimmed.to_string();

    if options.force_refresh {
        IMAGE_CACHE.lock().await.pop(&cache_key);
        IN_FLIGHT_DOWNLOADS.lock().await.remove(&cache_key);
    } else if let Some(cached_data_url) = lookup_cached_value(&cache_key).await {
        let hits = CACHE_HITS.fetch_add(1, Ordering::Relaxed) + 1;
        let misses = CACHE_MISSES.load(Ordering::Relaxed);
        info!(
            "{log_prefix} cache hit ({hits}/{total})",
            total = hits + misses
        );
        return Ok((*cached_data_url).clone());
    }

    let fetchable_url = media::to_fetchable_url(trimmed);
    if fetchable_url.is_empty() {
        return Err(AppError::invalid_params("image url cannot be empty"));
    }

    let (shared_future, is_creator) = {
        let mut in_flight = IN_FLIGHT_DOWNLOADS.lock().await;
        if let Some(existing) = in_flight.get(&cache_key) {
            (existing.clone(), false)
        } else {
            let misses = CACHE_MISSES.fetch_add(1, Ordering::Relaxed) + 1;
            let hits = CACHE_HITS.load(Ordering::Relaxed);
            info!(
                "{log_prefix} cache miss ({misses}/{total})",
                total = hits + misses
            );

            let fetchable_url = fetchable_url.clone();
            let log_prefix = log_prefix.to_string();
            let future = async move {
                download_image_as_base64(&fetchable_url, &log_prefix)
                    .await
                    .map(Arc::new)
            }
            .boxed()
            .shared();

            in_flight.insert(cache_key.clone(), future.clone());
            (future, true)
        }
    };

    if !is_creator {
        let hits = CACHE_HITS.fetch_add(1, Ordering::Relaxed) + 1;
        let misses = CACHE_MISSES.load(Ordering::Relaxed);
        info!(
            "{log_prefix} in-flight hit ({hits}/{total})",
            total = hits + misses
        );
    }

    let result = shared_future.await;
    if is_creator {
        IN_FLIGHT_DOWNLOADS.lock().await.remove(&cache_key);
    }

    let data_url = result?;
    if is_creator {
        insert_cache_value(cache_key, Arc::clone(&data_url)).await;
    }

    Ok((*data_url).clone())
}

pub async fn get_image_cache_stats() -> ImageCacheStats {
    let now = Instant::now();
    let cache = IMAGE_CACHE.lock().await;

    let mut valid_entries = 0usize;
    let mut total_size_bytes = 0usize;
    for (_, entry) in cache.iter() {
        if entry.expires_at > now {
            valid_entries += 1;
            total_size_bytes += entry.size_bytes;
        }
    }

    let cache_hits = CACHE_HITS.load(Ordering::Relaxed);
    let cache_misses = CACHE_MISSES.load(Ordering::Relaxed);
    ImageCacheStats {
        cache_size: cache.len(),
        valid_entries,
        total_size_kb: total_size_bytes.saturating_add(512) / 1024,
        cache_hits,
        cache_misses,
        hit_rate: compute_hit_rate(cache_hits, cache_misses),
        total_download_time_ms: TOTAL_DOWNLOAD_TIME_MS.load(Ordering::Relaxed),
    }
}

pub async fn clear_image_cache() {
    IMAGE_CACHE.lock().await.clear();
    IN_FLIGHT_DOWNLOADS.lock().await.clear();
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    TOTAL_DOWNLOAD_TIME_MS.store(0, Ordering::Relaxed);
    info!("{DEFAULT_LOG_PREFIX} cache cleared");
}

async fn lookup_cached_value(cache_key: &str) -> Option<Arc<String>> {
    let now = Instant::now();
    let mut cache = IMAGE_CACHE.lock().await;
    if let Some(entry) = cache.get(cache_key).cloned() {
        if entry.expires_at > now {
            return Some(entry.data_url);
        }
        cache.pop(cache_key);
    }
    None
}

async fn insert_cache_value(cache_key: String, data_url: Arc<String>) {
    let entry = CacheEntry {
        size_bytes: data_url.len(),
        expires_at: Instant::now() + CACHE_TTL,
        data_url,
    };
    IMAGE_CACHE.lock().await.put(cache_key, entry);
}

async fn cleanup_expired_cache() {
    let now = Instant::now();
    let mut cache = IMAGE_CACHE.lock().await;
    let before = cache.len();

    let stale_keys = cache
        .iter()
        .filter_map(|(key, entry)| {
            if entry.expires_at <= now {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for key in stale_keys {
        cache.pop(&key);
    }

    let cleaned = before.saturating_sub(cache.len());
    if cleaned > 0 {
        info!(
            "{DEFAULT_LOG_PREFIX} removed {cleaned} stale entries, remaining {}",
            cache.len()
        );
    }
}

fn ensure_cleanup_task_started() {
    if CLEANUP_TASK_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    tokio::spawn(async {
        let mut interval = time::interval(CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            cleanup_expired_cache().await;
        }
    });
}

fn compute_hit_rate(hits: u64, misses: u64) -> u64 {
    let total = hits + misses;
    if total == 0 {
        return 0;
    }
    ((hits as f64 / total as f64) * 100.0).round() as u64
}

fn elapsed_millis(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

async fn download_image_as_base64(image_url: &str, log_prefix: &str) -> Result<String, AppError> {
    let started_at = Instant::now();
    let url_for_log = truncate_for_log(image_url, DOWNLOAD_LOG_PREFIX_MAX_CHARS);
    info!("{log_prefix} downloading image: {url_for_log}");

    let response = match DOWNLOAD_HTTP_CLIENT
        .get(image_url)
        .header(USER_AGENT, IMAGE_DOWNLOADER_USER_AGENT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let duration_ms = elapsed_millis(started_at);
            error!("{log_prefix} download failed ({duration_ms}ms): {error}");
            return Err(AppError::internal(format!(
                "failed to download image from {image_url}: {error}"
            )));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let duration_ms = elapsed_millis(started_at);
        error!("{log_prefix} download failed ({duration_ms}ms): status {status}, body: {body}");
        return Err(AppError::internal(format!(
            "failed to download image from {image_url}: status {status}, body: {body}"
        )));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_string())
        .unwrap_or_else(|| "image/png".to_string());
    let bytes = response
        .bytes()
        .await
        .map_err(|error| AppError::internal(format!("failed to read image bytes: {error}")))?;
    let base64_data = STANDARD.encode(&bytes);

    let duration_ms = elapsed_millis(started_at);
    TOTAL_DOWNLOAD_TIME_MS.fetch_add(duration_ms, Ordering::Relaxed);
    let size_kb = bytes.len().saturating_add(512) / 1024;
    info!("{log_prefix} download completed: {size_kb}KB, {duration_ms}ms");

    Ok(format!("data:{content_type};base64,{base64_data}"))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use once_cell::sync::Lazy;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::Mutex,
    };

    use super::{
        GetImageBase64Options, clear_image_cache, get_image_base64_cached, get_image_cache_stats,
    };

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    async fn start_test_image_server(
        delay: Duration,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");
        let request_count = Arc::new(AtomicUsize::new(0));

        let request_count_for_task = Arc::clone(&request_count);
        let server_handle = tokio::spawn(async move {
            let response_body = vec![137, 80, 78, 71, 13, 10, 26, 10];

            loop {
                let (mut socket, _) = listener.accept().await.expect("accept should work");
                let request_count = Arc::clone(&request_count_for_task);
                let response_body = response_body.clone();

                tokio::spawn(async move {
                    let mut request_buffer = [0_u8; 1024];
                    let _ = socket.read(&mut request_buffer).await;
                    request_count.fetch_add(1, Ordering::SeqCst);

                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response_body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(&response_body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        (
            format!("http://127.0.0.1:{}/test.png", address.port()),
            request_count,
            server_handle,
        )
    }

    #[tokio::test]
    async fn cache_hit_uses_existing_download_result() {
        let _guard = TEST_MUTEX.lock().await;
        clear_image_cache().await;
        let (url, request_count, handle) = start_test_image_server(Duration::ZERO).await;

        let first = get_image_base64_cached(&url, GetImageBase64Options::default())
            .await
            .expect("first fetch should succeed");
        let second = get_image_base64_cached(&url, GetImageBase64Options::default())
            .await
            .expect("second fetch should hit cache");

        assert_eq!(first, second);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);

        let stats = get_image_cache_stats().await;
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.valid_entries, 1);

        handle.abort();
    }

    #[tokio::test]
    async fn concurrent_requests_share_in_flight_download() {
        let _guard = TEST_MUTEX.lock().await;
        clear_image_cache().await;
        let (url, request_count, handle) =
            start_test_image_server(Duration::from_millis(120)).await;

        let (first, second, third) = tokio::join!(
            get_image_base64_cached(&url, GetImageBase64Options::default()),
            get_image_base64_cached(&url, GetImageBase64Options::default()),
            get_image_base64_cached(&url, GetImageBase64Options::default()),
        );

        let first = first.expect("first concurrent request should succeed");
        let second = second.expect("second concurrent request should succeed");
        let third = third.expect("third concurrent request should succeed");
        assert_eq!(first, second);
        assert_eq!(second, third);
        assert_eq!(request_count.load(Ordering::SeqCst), 1);

        let stats = get_image_cache_stats().await;
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 1);

        handle.abort();
    }

    #[tokio::test]
    async fn force_refresh_bypasses_cache_entry() {
        let _guard = TEST_MUTEX.lock().await;
        clear_image_cache().await;
        let (url, request_count, handle) = start_test_image_server(Duration::ZERO).await;

        let _ = get_image_base64_cached(&url, GetImageBase64Options::default())
            .await
            .expect("initial request should succeed");
        let _ = get_image_base64_cached(
            &url,
            GetImageBase64Options {
                log_prefix: None,
                force_refresh: true,
            },
        )
        .await
        .expect("force refresh request should succeed");

        assert_eq!(request_count.load(Ordering::SeqCst), 2);
        let stats = get_image_cache_stats().await;
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 2);

        handle.abort();
    }
}
