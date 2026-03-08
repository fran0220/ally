# crates/core — 共享业务基础

所有业务逻辑的基础库，被 server/worker/watchdog 依赖。

## 模块结构

| 模块 | 职责 |
|------|------|
| `config/` | AppConfig 加载（dotenvy + config crate，环境变量驱动） |
| `auth/` | JWT 签发/验证 (`jwt.rs`) + Argon2 密码哈希 (`password.rs`) |
| `db/` | MySQL (SQLx) + Redis (deadpool-redis) 连接池创建 |
| `errors/` | 统一错误类型（thiserror），ErrorCategory 分类 |
| `llm/` | LLM 流式客户端（openai-compatible / gemini-compatible / anthropic） |
| `prompt_i18n/` | Prompt 国际化模板系统（PromptId/PromptCatalog/buildPrompt） |
| `capabilities/` | AI 模型能力目录（catalog 加载缓存、lookup 校验） |
| `runtime/` | DAG 执行引擎（graph_executor/pipeline_graph/task_bridge） |
| `generators/` | AI 内容生成（图片/视频/语音），按能力拆分模块 |
| `media/` | 媒体处理工具 |
| `api_config/` | 用户 API 配置管理 |
| `system/` | 系统信息（boot-id 等） |
| `billing/` | 计费系统（pricing/ledger/task 注册表） |

## generators 子模块

```
generators/
├── mod.rs     # 共享基础设施（retry, poll, http_client）+ 类型 + re-export
├── fal.rs     # fal.ai 共享（submit_task / poll_result / endpoint 映射）
├── image.rs   # generate_image → fal / openai-compatible / gemini-compatible
├── video.rs   # generate_video → fal (kling/veo) / openai-compatible / jimeng (seedance)
└── voice.rs   # generate_lip_sync (fal) / generate_voice_clone (fal) / create_voice_design (qwen)
```

## llm 模块

`stream_chat` 统一入口，按 provider_key 分发：
- `openai-compatible` → OpenAI SSE 协议（GPT / Grok / GLM）
- `gemini-compatible` → Gemini generateContent SSE
- `anthropic` → Anthropic `/v1/messages` SSE（content_block_delta 事件格式）

## 修改注意

- 修改 `pub` 接口需确认 server/worker/watchdog 中的调用点
- `config/mod.rs` 新增配置字段时同步更新 `.env.example`
- `errors/` 中新增错误类型需映射 HTTP 状态码（在 server/error.rs 中）
- 数据库查询使用 SQLx `query!` / `query_as!` 宏（编译期类型检查）
- generators 新增 provider 时需同步更新 `admin.rs` 白名单 + 前端 `validation.ts`
