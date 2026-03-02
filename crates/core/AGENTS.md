# crates/core — 共享业务基础

所有业务逻辑的基础库，被 server/worker/watchdog 依赖。

## 模块结构

| 模块 | 职责 |
|------|------|
| `config/` | AppConfig 加载（dotenvy + config crate，环境变量驱动） |
| `auth/` | JWT 签发/验证 (`jwt.rs`) + Argon2 密码哈希 (`password.rs`) |
| `db/` | MySQL (SQLx) + Redis (deadpool-redis) 连接池创建 |
| `errors/` | 统一错误类型（thiserror），ErrorCategory 分类 |
| `llm/` | LLM API 客户端封装 |
| `prompt_i18n/` | Prompt 国际化模板系统（PromptId/PromptCatalog/buildPrompt） |
| `capabilities/` | AI 模型能力目录（catalog 加载缓存、lookup 校验） |
| `runtime/` | DAG 执行引擎（graph_executor/pipeline_graph/task_bridge） |
| `generators/` | ID 生成器 |
| `media/` | 媒体处理工具 |
| `api_config/` | 用户 API 配置管理 |
| `system/` | 系统信息（boot-id 等） |

## 修改注意

- 修改 `pub` 接口需确认 server/worker/watchdog 中的调用点
- `config/mod.rs` 新增配置字段时同步更新 `.env.example`
- `errors/` 中新增错误类型需映射 HTTP 状态码（在 server/error.rs 中）
- 数据库查询使用 SQLx `query!` / `query_as!` 宏（编译期类型检查）
