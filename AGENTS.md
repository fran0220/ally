# Ally (waoowaoo-rust) — AGENTS.md

> Rust 全栈迁移项目：Axum HTTP + Redis Stream Worker + React/Vite 前端

## 项目架构

```
ally/
├── crates/
│   ├── core/       # 共享业务基础（config, auth, db, errors, llm, runtime）
│   └── app/        # 单一入口二进制（serve/work/watch 子命令）
├── frontend/       # React 19 + Vite 7 + TailwindCSS 4 + TanStack Query
├── deploy/         # 部署配置（灰度发布模板、CI 脚本）
├── migrations/     # SQL 迁移文件（SQLx MySQL）
├── scripts/        # 测试与验证脚本
├── standards/      # AI 能力/Prompt 标准数据
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
| 构建 | Cargo workspace（core + app，edition 2024） |

## 运行模式

```bash
# API（默认）
cargo run -p ally

# Worker
cargo run -p ally -- work

# Watchdog
cargo run -p ally -- watch
```

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

## AI 模型接入

通过 CLIProxyAPI 中转站 + jimeng-gateway 统一接入，6 种 Provider：

| Provider Key | 协议 | Base URL | 用途 |
|---|---|---|---|
| `openai-compatible` | OpenAI `/v1/chat/completions` | LLM Proxy `/v1` | LLM (GPT/Grok/GLM) + 图片 (gemini-flash-image-preview via chat) |
| `gemini-compatible` | Gemini `generateContent` | LLM Proxy | LLM (Gemini text) + 图片 (Gemini native) |
| `anthropic` | Anthropic `/v1/messages` | LLM Proxy | LLM (Claude sonnet/opus) |
| `fal` | FAL queue API | fal.ai 原生 | 视频 (kling/veo) + 唇形同步 + 语音克隆 |
| `qwen` | 阿里 DashScope | dashscope.aliyuncs.com | 语音设计 (TTS 定制) |
| `jimeng` | 异步任务 (POST + 轮询) | jpdata:5100 | 视频 (seedance-2.0) |

### 生成能力 → Provider 映射

| 能力 | 入口函数 | 支持的 Provider |
|---|---|---|
| 图片生成 | `generators::generate_image` | fal, openai-compatible, gemini-compatible |
| 视频生成 | `generators::generate_video` | fal (kling/veo), openai-compatible, jimeng (seedance) |
| 唇形同步 | `generators::generate_lip_sync` | fal |
| 语音克隆 | `generators::generate_voice_clone` | fal |
| 语音设计 | `generators::create_voice_design` | qwen |
| LLM 流式 | `llm::stream_chat` | openai-compatible, gemini-compatible, anthropic |

### 管理员 Provider 白名单

后端 `admin.rs` + 前端 `validation.ts` 同步维护：
`["fal", "qwen", "openai-compatible", "gemini-compatible", "anthropic", "jimeng"]`

### model_key 格式

`provider_id::model_id`，例如 `openai-compatible::gpt-5.4`、`jimeng::seedance-2.0`

## 部署目标

- 服务器：`jpdata`（185.200.65.233），SSH root 用户
- 基础设施：Docker (MySQL 8.0 + Redis 7) + Nginx 反代
- 部署模式：rsync + SSH 远程重启
- jpdata 上的 jimeng-gateway：systemd 服务，端口 5100，上游 jimeng-free-api Docker :8000

## 环境变量（必须）

参见 `.env.example`，关键变量：
- `DATABASE_URL` — MySQL 连接串
- `REDIS_URL` — Redis 连接串
- `JWT_SECRET` — JWT 签名密钥（≥32 字符）
- `CORS_ALLOW_ORIGIN` — 允许的前端域名（逗号分隔）
- `BILLING_ENABLED` — 是否启用计费（默认 false）

## 计费系统

极简积分制：**查价 × 用量 = 扣积分**，一个事务完成。

### 核心表

| 表 | 作用 |
|---|------|
| `model_pricing` | 单价表：`(api_type, model_id, unit)` → `unit_price` |
| `user_balances` | 用户余额：`balance` + `totalSpent` |
| `credit_records` | 所有流水：consume / recharge / refund / admin_adjust |

### 计费流程

```
任务提交（serve）→ 余额预检查（快速失败）→ 入队
任务完成（work）→ extract_billing_params → deduct_credits（原子扣减）
任务失败（work）→ 无操作（未扣过不用退）
```

### 关键代码

| 文件 | 职责 |
|------|------|
| `crates/core/src/billing/pricing.rs` | DB 查单价 `get_unit_price` |
| `crates/core/src/billing/task.rs` | 任务注册表 `BILLING_DEFS` + `extract_billing_params` |
| `crates/core/src/billing/ledger.rs` | `deduct_credits` / `add_credits` / `refund_credits` |
| `crates/core/src/billing/reporting.rs` | 费用汇总与流水查询 |

### 新增任务类型

只需在 `task.rs` 的 `BILLING_DEFS` 注册表中加一行：
```rust
defs.insert("new_task_type", IMAGE_DEF); // 或 VIDEO_DEF / TEXT_DEF / 自定义
```

### 新增模型定价

在 `model_pricing` 表 INSERT 一行即可，无需改代码。

## 子目录 AGENTS.md

各子目录包含更详细的上下文说明，修改对应目录时请参考。
