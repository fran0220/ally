use std::{env, fs, path::PathBuf};

use ab_glyph::{FontArc, PxScale};
use image::{DynamicImage, Rgba, RgbaImage, codecs::jpeg::JpegEncoder, imageops};
use imageproc::drawing::{draw_text_mut, text_size};
use once_cell::sync::OnceCell;
use tracing::warn;
use uuid::Uuid;

use crate::{
    errors::AppError,
    media::{
        download_source_bytes, download_storage_key_bytes, resolve_storage_key_from_media_value,
        upload_bytes_to_storage,
    },
};

const NEAR_BLACK_THRESHOLD: u8 = 30;
const BLACK_ROW_RATIO_THRESHOLD: f32 = 0.94;
const MIN_DETECTED_LABEL_ROWS: u32 = 8;
const LABEL_HEIGHT_RATIO: f32 = 0.06;
const LABEL_HEIGHT_MIN: u32 = 40;
const LABEL_HEIGHT_MAX: u32 = 120;
const LABEL_TEXT_PADDING_RATIO: f32 = 0.06;
const LABEL_TEXT_SCALE_RATIO: f32 = 0.5;
const LABEL_TEXT_SCALE_MIN: f32 = 14.0;
const LABEL_TEXT_SCALE_STEP: f32 = 2.0;
const JPEG_QUALITY: u8 = 90;

static LABEL_FONT: OnceCell<FontArc> = OnceCell::new();

#[derive(Debug, Clone, Default)]
pub struct UpdateImageLabelOptions {
    pub generate_new_key: bool,
    pub key_prefix: Option<String>,
}

impl UpdateImageLabelOptions {
    pub fn with_new_key(key_prefix: impl Into<String>) -> Self {
        Self {
            generate_new_key: true,
            key_prefix: Some(key_prefix.into()),
        }
    }
}

pub async fn update_image_label(
    image_url: &str,
    label_text: &str,
    options: Option<UpdateImageLabelOptions>,
) -> Result<String, AppError> {
    let source = image_url.trim();
    if source.is_empty() {
        return Err(AppError::invalid_params("image_url is required"));
    }

    let normalized_label = normalize_label_text(label_text)?;
    let options = options.unwrap_or_default();
    let original_key = resolve_storage_key_from_media_value(source);

    let image_bytes = if let Some(storage_key) = original_key.as_deref() {
        download_storage_key_bytes(storage_key).await?.0
    } else {
        download_source_bytes(source).await?.0
    };

    let processed = process_image_with_label(&image_bytes, &normalized_label)?;

    let target_key = if options.generate_new_key {
        build_generated_key(options.key_prefix.as_deref())
    } else {
        original_key.ok_or_else(|| {
            AppError::invalid_params(
                "image_url must resolve to a storage key when generate_new_key is false",
            )
        })?
    };

    upload_bytes_to_storage(&target_key, &processed).await?;
    Ok(target_key)
}

pub fn process_image_with_label(
    source_bytes: &[u8],
    label_text: &str,
) -> Result<Vec<u8>, AppError> {
    if source_bytes.is_empty() {
        return Err(AppError::invalid_params(
            "source image bytes cannot be empty",
        ));
    }

    let normalized_label = normalize_label_text(label_text)?;
    let decoded = image::load_from_memory(source_bytes)
        .map_err(|err| AppError::invalid_params(format!("failed to decode source image: {err}")))?;
    let original = decoded.to_rgba8();
    if original.width() == 0 || original.height() == 0 {
        return Err(AppError::invalid_params(
            "source image dimensions must be non-zero",
        ));
    }

    let existing_label_height = detect_existing_label_height(&original);
    let content_image = strip_existing_label(&original, existing_label_height);
    let label_height = compute_label_height(content_image.height());

    let mut output = RgbaImage::from_pixel(
        content_image.width(),
        content_image.height().saturating_add(label_height),
        Rgba([0, 0, 0, 255]),
    );
    imageops::replace(&mut output, &content_image, 0, i64::from(label_height));

    render_label_text(&mut output, label_height, &normalized_label)?;
    encode_jpeg(output)
}

fn normalize_label_text(label_text: &str) -> Result<String, AppError> {
    let trimmed = label_text.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_params("label_text is required"));
    }
    Ok(trimmed.to_string())
}

fn build_generated_key(prefix: Option<&str>) -> String {
    let normalized = prefix
        .map(|value| value.trim().trim_matches('/'))
        .filter(|value| !value.is_empty())
        .unwrap_or("labeled-image");
    format!("images/{normalized}-{}.jpg", Uuid::new_v4())
}

fn compute_label_height(content_height: u32) -> u32 {
    let scaled = (content_height as f32 * LABEL_HEIGHT_RATIO).round() as u32;
    let max_allowed = LABEL_HEIGHT_MAX.min(content_height.max(1));
    let min_allowed = LABEL_HEIGHT_MIN.min(max_allowed);
    scaled.max(min_allowed).min(max_allowed)
}

fn detect_existing_label_height(image: &RgbaImage) -> u32 {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return 0;
    }

    let max_scan_height = ((height as f32 * 0.2).round() as u32)
        .max(1)
        .min(height)
        .min(240);
    let sample_step = (width / 256).max(1);

    let mut detected_height = 0;
    for y in 0..max_scan_height {
        if is_near_black_row(image, y, sample_step) {
            detected_height = y + 1;
            continue;
        }
        break;
    }

    if detected_height < MIN_DETECTED_LABEL_ROWS {
        return 0;
    }

    let min_reasonable = ((height as f32 * 0.02).round() as u32).max(MIN_DETECTED_LABEL_ROWS);
    let max_reasonable = ((height as f32 * 0.2).round() as u32)
        .max(min_reasonable)
        .min(height);

    if detected_height < min_reasonable || detected_height > max_reasonable {
        return 0;
    }

    detected_height
}

fn is_near_black_row(image: &RgbaImage, y: u32, sample_step: u32) -> bool {
    let mut near_black = 0_u32;
    let mut samples = 0_u32;
    let width = image.width();
    let mut x = 0_u32;

    while x < width {
        let pixel = image.get_pixel(x, y).0;
        if pixel[3] > 127
            && pixel[0] < NEAR_BLACK_THRESHOLD
            && pixel[1] < NEAR_BLACK_THRESHOLD
            && pixel[2] < NEAR_BLACK_THRESHOLD
        {
            near_black += 1;
        }
        samples += 1;
        x = x.saturating_add(sample_step);
    }

    if samples == 0 {
        return false;
    }

    (near_black as f32 / samples as f32) >= BLACK_ROW_RATIO_THRESHOLD
}

fn strip_existing_label(image: &RgbaImage, existing_label_height: u32) -> RgbaImage {
    if existing_label_height > 0 && existing_label_height < image.height() {
        return imageops::crop_imm(
            image,
            0,
            existing_label_height,
            image.width(),
            image.height() - existing_label_height,
        )
        .to_image();
    }

    image.clone()
}

fn render_label_text(
    canvas: &mut RgbaImage,
    label_height: u32,
    label_text: &str,
) -> Result<(), AppError> {
    let font = load_font()?;

    let width = canvas.width();
    if width == 0 || label_height == 0 {
        return Ok(());
    }

    let horizontal_padding = ((width as f32) * LABEL_TEXT_PADDING_RATIO).round() as u32;
    let available_width = width
        .saturating_sub(horizontal_padding.saturating_mul(2))
        .max(1);

    let mut scale_value = (label_height as f32 * LABEL_TEXT_SCALE_RATIO)
        .max(LABEL_TEXT_SCALE_MIN)
        .min((label_height as f32 * 0.85).max(LABEL_TEXT_SCALE_MIN));

    let mut scale = PxScale::from(scale_value);
    let mut measured = text_size(scale, font, label_text);

    while measured.0 > available_width && scale_value > LABEL_TEXT_SCALE_MIN {
        scale_value = (scale_value - LABEL_TEXT_SCALE_STEP).max(LABEL_TEXT_SCALE_MIN);
        scale = PxScale::from(scale_value);
        measured = text_size(scale, font, label_text);
    }

    let mut text_x = (width.saturating_sub(measured.0) / 2) as i32;
    let min_x = horizontal_padding as i32;
    if text_x < min_x {
        text_x = min_x;
    }

    let max_x = width.saturating_sub(measured.0.saturating_add(horizontal_padding)) as i32;
    if text_x > max_x {
        text_x = max_x.max(0);
    }

    let text_y = (label_height.saturating_sub(measured.1) / 2) as i32;
    draw_text_mut(
        canvas,
        Rgba([255, 255, 255, 255]),
        text_x,
        text_y,
        scale,
        font,
        label_text,
    );

    Ok(())
}

fn load_font() -> Result<&'static FontArc, AppError> {
    LABEL_FONT.get_or_try_init(|| {
        let candidates = font_candidates();
        for candidate in &candidates {
            if !candidate.is_file() {
                continue;
            }

            let bytes = match fs::read(candidate) {
                Ok(bytes) => bytes,
                Err(err) => {
                    warn!(
                        font_path = %candidate.display(),
                        error = %err,
                        "failed to read label font"
                    );
                    continue;
                }
            };

            match FontArc::try_from_vec(bytes) {
                Ok(font) => return Ok(font),
                Err(err) => {
                    warn!(
                        font_path = %candidate.display(),
                        error = %err,
                        "failed to parse label font"
                    );
                }
            }
        }

        let attempted = candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(AppError::internal(format!(
            "unable to load label font; set LABEL_FONT_PATH to a readable CJK font file. candidates: {attempted}"
        )))
    })
}

fn font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var("LABEL_FONT_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }

    candidates.push(PathBuf::from(
        "./allyvideo/src/assets/fonts/NotoSansSC-Regular.ttf",
    ));
    candidates.push(PathBuf::from("./src/assets/fonts/NotoSansSC-Regular.ttf"));
    candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ));
    candidates.push(PathBuf::from(
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ));
    candidates.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
    candidates.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
    candidates.push(PathBuf::from("/System/Library/Fonts/STHeiti Medium.ttc"));
    candidates.push(PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));

    let mut deduped = Vec::new();
    for path in candidates {
        if !deduped.iter().any(|existing| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

fn encode_jpeg(image: RgbaImage) -> Result<Vec<u8>, AppError> {
    let rgb = DynamicImage::ImageRgba8(image).to_rgb8();
    let mut output = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut output, JPEG_QUALITY);
    encoder.encode_image(&rgb).map_err(|err| {
        AppError::internal(format!("failed to encode labeled image as jpeg: {err}"))
    })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{compute_label_height, detect_existing_label_height, strip_existing_label};
    use image::{Rgba, RgbaImage};

    #[test]
    fn compute_label_height_clamps_by_ratio_and_bounds() {
        assert_eq!(compute_label_height(2_000), 120);
        assert_eq!(compute_label_height(1_000), 60);
        assert_eq!(compute_label_height(600), 40);
        assert_eq!(compute_label_height(20), 20);
    }

    #[test]
    fn detect_existing_label_height_finds_black_bar_from_top() {
        let mut image = RgbaImage::from_pixel(120, 100, Rgba([255, 255, 255, 255]));
        for y in 0..18 {
            for x in 0..120 {
                image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }

        assert_eq!(detect_existing_label_height(&image), 18);
    }

    #[test]
    fn detect_existing_label_height_ignores_non_black_headers() {
        let mut image = RgbaImage::from_pixel(120, 100, Rgba([255, 255, 255, 255]));
        for y in 0..18 {
            for x in 0..120 {
                image.put_pixel(x, y, Rgba([40, 40, 40, 255]));
            }
        }

        assert_eq!(detect_existing_label_height(&image), 0);
    }

    #[test]
    fn strip_existing_label_removes_detected_header_pixels() {
        let mut image = RgbaImage::from_pixel(64, 80, Rgba([220, 0, 0, 255]));
        for y in 0..12 {
            for x in 0..64 {
                image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }

        let stripped = strip_existing_label(&image, 12);
        assert_eq!(stripped.height(), 68);
        assert_eq!(stripped.get_pixel(0, 0).0, [220, 0, 0, 255]);
    }
}
