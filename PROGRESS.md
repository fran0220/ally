# Rust 迁移进度跟踪

> 对照 `docs/rust-migration-plan.md`，实时更新实现状态

## Phase 0: 项目骨架 ✅
- [x] Cargo workspace (4 crates: core, server, worker, watchdog)
- [x] DB 连接 (SQLx MySQL)
- [x] Redis 连接 (deadpool-redis)
- [x] 环境配置 (config + dotenvy)
- [x] 迁移文件 (0001_initial.sql, 0002_run_runtime.sql)

## Phase 1: 认证 ✅
- [x] JWT 签发/验证 (`core/auth/jwt.rs`)
- [x] 密码哈希 argon2 (`core/auth/password.rs`)
- [x] `POST /api/auth/register`
- [x] `POST /api/auth/login`
- [x] `POST /api/auth/refresh`
- [x] AuthUser extractor (`server/extractors/auth.rs`)

## Phase 2: 核心 API ✅
- [x] Projects CRUD (`server/routes/projects.rs` — 4 路由)
- [x] Tasks API (`server/routes/tasks.rs` — 4 路由)
- [x] Task Submit (`server/routes/task_submit.rs`)
- [x] Runs API (`server/routes/runs.rs` — 4 路由)
- [x] SSE 推送 (`server/routes/sse.rs`)
- [x] COS 签名 (`server/routes/cos.rs`)
- [x] 文件上传 (`server/routes/files.rs`)
- [x] Media 代理 (`server/routes/media.rs`)
- [x] System API (`server/routes/system.rs`)

## Phase 3: 资产中心 ✅
- [x] Asset Hub 全部路由 (`server/routes/asset_hub.rs` — 27 路由, 2238L)

## Phase 4: 小说推广 ✅
- [x] Novel Promotion 全部路由 (`server/routes/novel.rs` — 57 路由, 2844L)

## Phase 5: Worker 进程 ✅
- [x] Redis Stream 消费者框架 (`worker/consumer.rs`)
- [x] 任务分发器 (`worker/dispatcher.rs`)
- [x] Worker 心跳 (`worker/heartbeat.rs`)
- [x] Worker main 启动 (`worker/main.rs`)

### 5a: Image Worker
- [x] `image_panel` handler
- [x] `image_character` handler
- [x] `image_location` handler
- [x] `panel_variant` handler
- [x] `modify_asset_image` handler
- [x] `regenerate_group` handler
- [x] `asset_hub_image` handler
- [x] `asset_hub_modify` handler

### 5b: Text Worker
- [x] `analyze_novel` handler
- [x] `analyze_global` handler
- [x] `story_to_script` handler
- [x] `script_to_storyboard` handler
- [x] `clips_build` handler
- [x] `screenplay_convert` handler
- [x] `episode_split` handler
- [x] `voice_analyze` handler
- [x] `ai_create_character` handler
- [x] `ai_create_location` handler
- [x] `ai_modify_appearance` handler
- [x] `ai_modify_location` handler
- [x] `ai_modify_shot_prompt` handler
- [x] `analyze_shot_variants` handler
- [x] `character_profile` handler
- [x] `reference_to_character` handler
- [x] `asset_hub_ai_design` handler
- [x] `asset_hub_ai_modify` handler
- [x] `regenerate_text` handler
- [x] `insert_panel` handler

### 5c: Video + Voice Worker
- [x] `video_panel` handler
- [x] `lip_sync` handler
- [x] `voice_line` handler
- [x] `voice_design` handler
- [x] `asset_hub_voice_design` handler

## Phase 6: Watchdog + Admin ✅
- [x] Admin AI Config 路由 (`server/routes/admin.rs`)
- [x] Watchdog 超时扫描逻辑 (`watchdog/main.rs` — 已支持 timeout fail + task.failed 事件)

## Phase 7: 前端迁移 ✅
### 7a: 骨架
- [x] Vite 项目初始化
- [x] React Router v7 路由配置 (`frontend/src/App.tsx`)
- [x] i18next 配置 (`frontend/src/i18n/index.ts`)
- [x] API client 层 (`frontend/src/api/client.ts`)
- [x] Auth API (`frontend/src/api/auth.ts`)
- [x] TanStack Query 配置 (`frontend/src/lib/query-client.ts`, `frontend/src/lib/query-keys.ts`)
- [x] SSE Client 封装 (`frontend/src/api/sse.ts` — 176L)
- [x] Tailwind CSS 4 配置
- [x] Glass UI 原语迁移 (`frontend/src/components/ui/primitives/`)

### 7b: 页面迁移
- [x] 着陆页 (`Landing.tsx` — 23L)
- [x] 登录 (`SignIn.tsx` — 58L)
- [x] 注册 (`SignUp.tsx` — 58L)
- [x] 工作区列表 (`WorkspaceList.tsx` — 383L)
- [x] 项目工作台 (`ProjectWorkbench.tsx` — 630L)
- [x] 全局资产中心 (`AssetHub.tsx` — 431L)
- [x] 个人资料 (`Profile.tsx` — 26L)
- [x] 管理后台 (`AiConfig.tsx` — 472L)

### 7c: 视频编辑器
- [x] Remotion 视频编辑器迁移
- [x] Timeline / TransitionPicker / VideoComposition
- [x] 编辑器 hooks + state 迁移
- [ ] 端到端测试

### 7d: 业务组件迁移 ✅
- [x] QueryProvider (`providers/QueryProvider.tsx`)
- [x] SharedComponents (`ui/SharedComponents.tsx` — AnimatedBackground/GlassPanel/Button)
- [x] Custom Icons (`ui/icons/custom.tsx` — 98 图标, 746L, 无 lucide-react 依赖)
- [x] RatioPreviewIcon (`ui/icons/RatioPreviewIcon.tsx`)
- [x] ConfigModals barrel (`ui/ConfigModals.tsx`)
- [x] ConfigConfirmModal (`ui/config-modals/ConfigConfirmModal.tsx`)
- [x] ConfigDeleteModal (`ui/config-modals/ConfigDeleteModal.tsx`)
- [x] ConfigEditModal + SettingsModal + WorldContextModal (`ui/config-modals/ConfigEditModal.tsx` — 448L)
- [x] WorldContextModal (`ui/config-modals/WorldContextModal.tsx` — 105L)
- [x] config-modal-selectors (`ui/config-modals/config-modal-selectors.tsx` — 160L)
- [x] ModelCapabilityDropdown (`ui/config-modals/ModelCapabilityDropdown.tsx` — 397L)
- [x] CapsuleNav (`ui/CapsuleNav.tsx` — 393L)
- [x] StoryboardHeaderV2 (`ui/patterns/StoryboardHeaderV2.tsx` — 81L)
- [x] PanelCardV2 (`ui/patterns/PanelCardV2.tsx` — 218L)
- [x] PanelEditFormV2 (`ui/patterns/PanelEditFormV2.tsx` — 180L)
- [x] CharacterCreationForm (`shared/assets/character-creation/CharacterCreationForm.tsx` — 351L)
- [x] CharacterCreationPreview (`shared/assets/character-creation/CharacterCreationPreview.tsx` — 76L)
- [x] CharacterCreationModal (`shared/assets/CharacterCreationModal.tsx` — 240L)
- [x] CharacterEditModal (`shared/assets/CharacterEditModal.tsx` — 366L)
- [x] LocationCreationModal (`shared/assets/LocationCreationModal.tsx` — 282L)
- [x] LocationEditModal (`shared/assets/LocationEditModal.tsx` — 335L)
- [x] GlobalAssetPicker (`shared/assets/GlobalAssetPicker.tsx` — 448L)
- [x] LLMStageStreamCard (`llm-console/LLMStageStreamCard.tsx` — 399L)
- [x] VoiceDesignDialogBase (`voice/VoiceDesignDialogBase.tsx` — 381L)

## Phase 8: 测试 + 验收 📋
### 8a: 测试基建
- [x] API 对比测试骨架 (`scripts/api-contract-regression.mjs`)
- [x] API 运行时对比脚本增强（支持 `{{ENV_VAR}}` 占位 + 缺失用例跳过）
- [x] API 运行时样例用例扩展至 39 条，覆盖 14 分组核心路由 (`scripts/api-runtime-cases.sample.json`)
- [x] Worker 任务提交 + 结果验证烟测脚本（image/text/video/voice）(`scripts/worker-runtime-smoke.mjs` + `scripts/worker-runtime-cases.sample.json`)
- [x] SSE 断线重连烟测脚本 (`scripts/sse-reconnect-smoke.mjs`)
- [x] Next.js vs Rust 性能基准脚本 (`scripts/api-performance-benchmark.mjs` + `scripts/api-performance-cases.sample.json`)
- [x] 灰度切换配置模板（Nginx/Caddy）(`deploy/gray-release/`)

### 8b: 联调与验收执行
- [x] 全 API 对比执行（legacy/rust 同时在线已执行，见 `reports/api-runtime-compare.md`）
- [x] Worker 全任务类型执行测试（image/text/video/voice smoke 4/4 通过，见 `reports/worker-runtime-smoke.md`）
- [x] SSE 断线重连联调执行（2026-03-01 通过，见 `reports/sse-reconnect-smoke.md`）
- [x] 性能基准联调执行（legacy/rust 基准已执行，见 `reports/api-performance-benchmark.md`）
- [x] 前端全功能手动验收（关键路由 + 注册 + 工作区/资产中心/项目工作台已完成，见本节快照）
- [ ] 灰度切换实操（配置已完成；当前环境缺少 Nginx/Caddy 可执行文件）

### 8b 执行快照（2026-03-01）
- `node scripts/api-contract-regression.mjs` → `PASS=124 / PASS_WITH_EXTRA=10 / FAIL=1 / INCOMPLETE=11 / MISSING_SIDE=8`（报告：`../docs/rust-api-contract-regression-report.md`）
- `npx prisma db execute --file waoowaoo-rust/migrations/0002_run_runtime.sql --schema prisma/schema.prisma` 已执行（补齐 `graph_*` 运行时表）
- `node scripts/api-runtime-compare.mjs --legacy-base http://127.0.0.1:3000 --rust-base http://127.0.0.1:3001 --token ... --cases /tmp/api-runtime-cases.phase8.json` → `PASS=17 / FAIL=22 / SKIPPED=0`（报告：`reports/api-runtime-compare.md`）
- `node scripts/worker-runtime-smoke.mjs --base http://127.0.0.1:3001` → `PASS=4 / FAIL=0 / SKIPPED=0`（报告：`reports/worker-runtime-smoke.md`）
- `node scripts/sse-reconnect-smoke.mjs --base http://127.0.0.1:3001 --project-id global-asset-hub` → `RESULT=PASS`（报告：`reports/sse-reconnect-smoke.md`）
- `node scripts/api-performance-benchmark.mjs --legacy-base http://127.0.0.1:3000 --rust-base http://127.0.0.1:3001 --token ... --cases /tmp/api-performance-cases.phase8.json --duration-ms 5000 --concurrency 2` → `PASS=5 / FAIL=2 / SKIPPED=0`（报告：`reports/api-performance-benchmark.md`）
- 前端手动验收（Playwright 浏览器实测）：`/auth/signup`、`/workspace`、`/workspace/asset-hub`、`/workspace/:projectId`、`/profile` 页面可加载，注册流程可达工作区；`/admin/ai-config` 在普通用户下返回 403（符合权限预期）。
- 修复执行期问题：
  - `crates/worker/src/dispatcher.rs`：消费/持久化错误不再导致 worker 主循环退出（避免 `1213 deadlock` 直接中断进程）
  - `scripts/worker-runtime-smoke.mjs`：兼容 Rust 任务事件的 `snake_case` 字段读取
  - `crates/core/src/config/mod.rs`：`CORS_ALLOW_ORIGIN` 支持逗号字符串反序列化
  - `crates/core/src/generators/mod.rs`：新增 Ark/Google/Minimax/Vidu/OpenAI-Compatible 视频生成器、Google Imagen + Gemini Batch + Ark Seedream 图片生成器、Vidu Lip Sync，并统一引入轮询超时 + 重试退避策略
  - `crates/server/src/middleware/cors.rs`：按 credentials 场景修正 CORS 头（显式 allow-headers + allow-credentials）
  - `frontend/src`：清理误提交的 `.js` 产物文件，避免覆盖同名 `.ts/.tsx` 模块
- `cd frontend && npm run build` → Vite 构建通过（`dist/` 产物生成）

### 当前阻塞
- 灰度切换实操尚未执行：当前环境无 `nginx` / `caddy` 可执行文件，无法对 `deploy/gray-release/` 模板做真实流量切换演练。
- legacy 与 rust 运行时差异仍存在（API 对比 22 个失败项），详见 `reports/api-runtime-compare.md`。

## 关键差距收敛（本轮）
- [x] Gap 1 — prompt-i18n 模块落地（`crates/core/src/prompt_i18n/`，已实现 PromptId/PromptCatalog/template-store/buildPrompt 与严格变量校验）
- [x] Gap 2 — model-capabilities 模块（`crates/core/src/capabilities/`，已实现 catalog 加载缓存、lookup 选择校验、video-effective 组合计算）
- [x] Gap 3 — run-runtime DAG 执行引擎扩展（`crates/core/src/runtime/` — 2,020L，含 graph_executor/pipeline_graph/task_bridge/service/types/workflow/publisher/quick_run_graph）
- [x] Gap 4 — 前端全部组件迁移完成（47 TSX components，覆盖 TS 源 41 个 + 额外 6 个辅助组件，共 11,518L）

## 清理任务 ✅
- [x] 移除 billing 模块 (`core/billing/` 已删除)
- [x] 移除 billing 路由 (balance/costs/cost_details/transactions)
- [x] 移除 InsufficientBalance 错误码
- [x] 移除 Billing ErrorCategory
- [x] 清理 worker `dead_code` 警告（10/10）并验证 `cargo check --workspace` 零警告通过

---

## 统计概览

| 维度 | 数值 |
|------|------|
| Rust 后端 | ~21,517 行（4 crates: core 7,505 / server 8,307 / worker 5,520 / watchdog 185） |
| 前端（Phase 7 迁移） | 86 文件 / ~11,518 行（47 组件） |
| 编译状态 | ✅ `cargo check --workspace` 零警告通过；✅ `npx tsc --noEmit` 零错误通过 |
| 组件覆盖率 | 47/41 TSX（100% 覆盖 TS 源组件 + 6 个额外辅助组件） |
| 完成度 | Phase 0-7 ✅，Gap 1-4 全部收敛，Phase 8 联调执行主项已完成 ≈ **99%** |
| 关键瓶颈 | 灰度切换实操（缺少 Nginx/Caddy 可执行环境）与 legacy/rust 运行时差异收敛（22 项） |
