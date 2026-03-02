# Ally (waoowaoo-rust) — AGENTS.md

> Rust 全栈迁移项目：Axum HTTP + Redis Stream Worker + React/Vite 前端

## 项目架构

```
ally/
├── crates/
│   ├── core/       # 共享业务基础（config, auth, db, errors, llm, runtime）
│   ├── server/     # Axum HTTP API 服务（routes, middleware, SSE）
│   ├── worker/     # Redis Stream 消费者进程（image/text/video/voice handlers）
│   └── watchdog/   # 超时扫描进程
├── frontend/       # React 19 + Vite 7 + TailwindCSS 4 + TanStack Query
├── deploy/         # 部署配置（灰度发布模板、CI 脚本）
├── migrations/     # SQL 迁移文件（SQLx MySQL）
├── scripts/        # 测试与验证脚本
├── standards/      # AI 能力/定价/Prompt 标准数据
└── messages/       # 消息模板
```

## 技术栈

| 层 | 技术 |
|---|------|
| HTTP 框架 | Axum 0.8 + Tower 中间件 |
| 数据库 | MySQL 8.0 (SQLx, deadpool) |
| 缓存/队列 | Redis 7 (deadpool-redis, Redis Streams) |
| 认证 | JWT (jsonwebtoken) + Argon2 密码哈希 |
| 前端 | React 19 + React Router 7 + Vite 7 + TailwindCSS 4 |
| 国际化 | i18next |
| 数据获取 | TanStack Query v5 |
| 构建 | Cargo workspace (edition 2024) |

## 验证命令（必须在提交前通过）

```bash
# Rust 后端
cargo fmt                              # 格式化
cargo check --workspace                # 类型检查
cargo clippy --workspace --all-targets # Lint
cargo test --workspace                 # 测试

# 前端
cd frontend && npx tsc --noEmit        # TypeScript 类型检查
cd frontend && npm run build           # Vite 构建
```

## 代码规范

### Rust
- Edition 2024，使用 workspace dependencies
- 错误处理：`anyhow::Result` 用于应用层，`thiserror` 用于库层（core crate）
- 异步运行时：Tokio（macros, rt-multi-thread）
- 日志：`tracing` + `tracing-subscriber`（JSON 格式输出）
- 配置：`config` crate + `dotenvy`，环境变量驱动
- 数据库：SQLx 原生查询（不用 ORM），类型安全的 `query!` / `query_as!`

### 前端
- React 19 函数组件 + Hooks
- 路由：React Router v7
- 状态管理：TanStack Query（服务端状态） + React Context（客户端状态）
- 样式：TailwindCSS 4（@tailwindcss/vite 插件）
- 文件命名：PascalCase 组件文件，camelCase 工具文件

## 部署目标

- 服务器：`jpdata`（185.200.65.233），SSH root 用户
- 基础设施：Docker (MySQL 8.0 + Redis 7) + PM2 (Node.js) + Nginx 反代
- 部署模式：rsync + SSH 远程重启

## 环境变量（必须）

参见 `.env.example`，关键变量：
- `DATABASE_URL` — MySQL 连接串
- `REDIS_URL` — Redis 连接串
- `JWT_SECRET` — JWT 签名密钥（≥32 字符）
- `CORS_ALLOW_ORIGIN` — 允许的前端域名（逗号分隔）

## 子目录 AGENTS.md

各子目录包含更详细的上下文说明，修改对应目录时请参考。
