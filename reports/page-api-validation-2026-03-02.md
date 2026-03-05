# 页面功能与后端 API 对齐验证（2026-03-02）

## 验证任务清单

1. 盘点前端页面路由与页面级功能入口
2. 逐页映射前端调用到后端 API（路径 + 方法）
3. 逐条核验后端是否实现对应路由与处理逻辑
4. 修复发现的前后端不一致
5. 运行 Rust + 前端编译校验，确认无回归

## 逐页验证结果

### 1) `/` Landing
- 功能：静态落地页
- 后端 API：无
- 结论：通过

### 2) `/auth/signin` 登录页
- 前端调用：`POST /api/auth/login`
- 后端实现：`auth.rs::login`
- 结论：通过

### 3) `/auth/signup` 注册页
- 前端调用：`POST /api/auth/register`
- 后端实现：`auth.rs::register`
- 结论：通过

### 4) `/workspace` 项目列表页
- 前端调用：
  - `GET /api/projects`
  - `POST /api/projects`
  - `PATCH /api/projects/{id}`
  - `DELETE /api/projects/{id}`
- 后端实现：`projects.rs::{list,create,update,delete}`
- 结论：通过

### 5) `/workspace/:projectId` 项目工作台页（ProjectWorkbench）
- 前端调用：
  - `GET /api/novel-promotion/{projectId}`
  - `GET/POST /api/novel-promotion/{projectId}/episodes`
  - `GET/PATCH/DELETE /api/novel-promotion/{projectId}/episodes/{episodeId}`
  - `POST /api/novel-promotion/{projectId}/episodes/split-by-markers`
  - `GET /api/novel-promotion/{projectId}/storyboards`
  - `POST /api/novel-promotion/{projectId}/video-urls`
  - `GET /api/novel-promotion/{projectId}/voice-lines`
  - `GET/PUT /api/novel-promotion/{projectId}/editor`
  - `GET /api/tasks`
  - `GET /api/sse`
  - `GET /api/novel-promotion/{projectId}/download-images`
  - `POST /api/novel-promotion/{projectId}/download-videos`
  - `GET /api/novel-promotion/{projectId}/download-voices`
- 后端实现：`novel.rs`（显式路由 + `dispatch`）、`tasks.rs`、`sse.rs`
- 结论：通过

### 6) `/workspace/asset-hub` 全局资产页
- 前端调用：
  - 文件夹：`GET/POST /api/asset-hub/folders`, `PATCH/DELETE /api/asset-hub/folders/{folderId}`
  - 角色：`GET/POST /api/asset-hub/characters`, `GET/PATCH/DELETE /api/asset-hub/characters/{characterId}`
  - 角色形象：`PATCH /api/asset-hub/characters/{characterId}/appearances/{appearanceIndex}`
  - 场景：`GET/POST /api/asset-hub/locations`, `GET/PATCH/DELETE /api/asset-hub/locations/{locationId}`
  - 声音：`GET/POST /api/asset-hub/voices`, `PATCH/DELETE /api/asset-hub/voices/{id}`
  - 生成图：`POST /api/asset-hub/generate-image`
  - SSE：`GET /api/sse?projectId=global-asset-hub`
- 后端实现：`asset_hub.rs`、`sse.rs`（全局资产 hub 已特殊放行）
- 结论：通过

### 7) `/profile` 个人配置页
- 前端调用：
  - `GET /api/user-preference`
  - `GET /api/user/models`
- 后端实现：`user.rs::{get_preference,models}`
- 结论：通过

### 8) `/admin/ai-config` 管理配置页
- 前端调用：
  - `GET /api/admin/ai-config`
  - `PUT /api/admin/ai-config`
- 后端实现：`admin.rs::{get,update}`（Admin 鉴权）
- 结论：通过（依赖管理员身份）

## 本轮修复项

### 修复 1：图片预览 COS 路径不一致
- 问题：前端媒体工具使用 `/api/cos/sign`，Rust 后端主路由为 `/api/cos/image`
- 修复：
  - 前端切换到 `/api/cos/image`
  - 后端增加 `/api/cos/sign` 兼容别名，避免旧链接失效
- 修改文件：
  - `frontend/src/lib/media/image-url.ts`
  - `crates/server/src/routes/cos.rs`

## 本轮校验命令结果

- `cargo check -p waoowaoo-server`：通过
- `cd frontend && npx tsc --noEmit`：通过

## 说明（非页面阻塞项）

- 静态 contract 脚本仍会把二进制流接口（zip/file/proxy）标为 `INCOMPLETE`，这是键级 JSON 对比的限制，不代表页面不可用。
- 计费相关接口按项目要求移除，不纳入页面功能缺口。
