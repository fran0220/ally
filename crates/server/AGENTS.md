# crates/server — Axum HTTP API

基于 Axum 0.8 的 HTTP API 服务，入口 `main.rs`。

## 目录结构

| 目录/文件 | 职责 |
|-----------|------|
| `main.rs` | 服务启动：tracing 初始化 → 配置加载 → DB/Redis 连接 → Router 构建 → 监听 |
| `app_state.rs` | AppState（Config + MySqlPool + RedisPool） |
| `error.rs` | 错误到 HTTP 响应的转换 |
| `routes/mod.rs` | 路由注册总入口（api_router 函数） |
| `routes/*.rs` | 各业务路由模块（auth, projects, tasks, runs, sse, cos, files, media, asset_hub, novel, admin, user） |
| `extractors/` | Axum 自定义 Extractor（AuthUser 认证提取器） |
| `middleware/` | 中间件（CORS, 错误处理, 日志追踪） |
| `sse/` | Server-Sent Events 推送 |

## 路由约定

- 路由前缀统一 `/api/`，健康检查 `/healthz`
- 认证路由：使用 `AuthUser` extractor（`extractors/auth.rs`）
- 新增路由模块：在 `routes/mod.rs` 中 `pub mod` + `.merge(module::router())`
- 每个路由模块暴露 `pub fn router() -> Router<AppState>`

## 中间件栈

```
请求 → CORS → tracing → 路由匹配 → AuthUser 提取 → Handler → 响应
```

## 端口

默认 `PORT=3001`（配置在 `.env`）
