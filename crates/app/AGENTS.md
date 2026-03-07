# crates/app — Unified Binary

单二进制入口 `ally`，通过子命令运行不同模式：

```bash
ally serve   # Axum HTTP API
ally work    # Redis Stream Worker
ally watch   # Watchdog 超时扫描
```

## 目录结构

| 目录/文件 | 职责 |
|-----------|------|
| `src/main.rs` | CLI dispatch + tracing 初始化 + config/db/redis 初始化 |
| `src/app_state.rs` | API AppState（Config + MySqlPool + RedisPool） |
| `src/error.rs` | API 错误到 HTTP 响应转换 |
| `src/extractors/` | API 自定义 Extractor（AuthUser 等） |
| `src/middleware/` | API 中间件（CORS、日志、request id） |
| `src/routes/` | API 路由模块 |
| `src/worker/` | Worker 子模块（consumer/dispatcher/handlers/heartbeat/runtime/task_context） |
| `src/watchdog.rs` | Watchdog 扫描与恢复逻辑 |

## Serve 路由约定

- 路由前缀统一 `/api/`，健康检查 `/healthz`
- 认证路由使用 `AuthUser` extractor（`extractors/auth.rs`）
- 新增路由模块：在 `routes/mod.rs` 中 `pub mod` + `.merge(module::router())`
- 每个路由模块暴露 `pub fn router() -> Router<AppState>`

## Work 队列并发

| 队列 | 默认并发 | 环境变量 |
|------|---------|----------|
| image | 20 | `QUEUE_CONCURRENCY_IMAGE` |
| text | 10 | `QUEUE_CONCURRENCY_TEXT` |
| video | 4 | `QUEUE_CONCURRENCY_VIDEO` |
| voice | 10 | `QUEUE_CONCURRENCY_VOICE` |

Worker 的 `runtime.rs` 使用 `OnceCell<MySqlPool>` 保存全局运行时连接，不要改为多实例初始化。

## Watch 子命令

- 入口：`watchdog::run_watchdog(mysql, redis)`
- 默认参数：
  - `WATCHDOG_INTERVAL_MS=30000`
  - `TASK_HEARTBEAT_TIMEOUT_MS=90000`
  - `WATCHDOG_BATCH_LIMIT=100`
