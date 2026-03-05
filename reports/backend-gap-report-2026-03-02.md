# Rust 迁移差距检查（逐文件）

- 检查日期: 2026-03-02
- 对比范围: `/Users/fan/ally` vs `/Users/fan/ally/allyvideo` 后端实现
- 口径: 计费相关接口和逻辑按“已移除”处理，不计为缺失

## 1) API 路由逐文件对照（121 文件）

- implemented: 106
- missing: 9
- billing_excluded: 5
- no_method (NextAuth 特殊): 1

| 原始文件 | Endpoint | 方法 | 状态 | Rust 对应/备注 |
|---|---|---|---|---|
| allyvideo/src/app/api/asset-hub/ai-design-character/route.ts | /api/asset-hub/ai-design-character | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/ai-design-location/route.ts | /api/asset-hub/ai-design-location | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/ai-modify-character/route.ts | /api/asset-hub/ai-modify-character | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/ai-modify-location/route.ts | /api/asset-hub/ai-modify-location | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/appearances/route.ts | /api/asset-hub/appearances | DELETE,PATCH,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/character-voice/route.ts | /api/asset-hub/character-voice | PATCH,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/characters/[characterId]/appearances/[appearanceIndex]/route.ts | /api/asset-hub/characters/{characterId}/appearances/{appearanceIndex} | DELETE,PATCH,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/characters/[characterId]/route.ts | /api/asset-hub/characters/{characterId} | DELETE,GET,PATCH | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/characters/route.ts | /api/asset-hub/characters | GET,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/folders/[folderId]/route.ts | /api/asset-hub/folders/{folderId} | DELETE,PATCH | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/folders/route.ts | /api/asset-hub/folders | GET,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/generate-image/route.ts | /api/asset-hub/generate-image | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/locations/[locationId]/route.ts | /api/asset-hub/locations/{locationId} | DELETE,GET,PATCH | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/locations/route.ts | /api/asset-hub/locations | GET,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/modify-image/route.ts | /api/asset-hub/modify-image | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/picker/route.ts | /api/asset-hub/picker | GET | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/reference-to-character/route.ts | /api/asset-hub/reference-to-character | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/select-image/route.ts | /api/asset-hub/select-image | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/undo-image/route.ts | /api/asset-hub/undo-image | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/update-asset-label/route.ts | /api/asset-hub/update-asset-label | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/upload-image/route.ts | /api/asset-hub/upload-image | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/upload-temp/route.ts | /api/asset-hub/upload-temp | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/voice-design/route.ts | /api/asset-hub/voice-design | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/voices/[id]/route.ts | /api/asset-hub/voices/{id} | DELETE,PATCH | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/voices/route.ts | /api/asset-hub/voices | GET,POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/asset-hub/voices/upload/route.ts | /api/asset-hub/voices/upload | POST | implemented | crates/server/src/routes/asset_hub.rs |
| allyvideo/src/app/api/auth/[...nextauth]/route.ts | /api/auth/{*nextauth} | - | no_method | NextAuth 路由未迁移（Rust 已改为 JWT register/login/refresh） |
| allyvideo/src/app/api/auth/register/route.ts | /api/auth/register | POST | implemented | crates/server/src/routes/mod.rs + crates/server/src/routes/auth.rs |
| allyvideo/src/app/api/cos/image/route.ts | /api/cos/image | GET | implemented | crates/server/src/routes/cos.rs |
| allyvideo/src/app/api/files/[...path]/route.ts | /api/files/{*path} | GET | implemented | crates/server/src/routes/files.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/ai-create-character/route.ts | /api/novel-promotion/{projectId}/ai-create-character | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/ai-create-location/route.ts | /api/novel-promotion/{projectId}/ai-create-location | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/ai-modify-appearance/route.ts | /api/novel-promotion/{projectId}/ai-modify-appearance | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/ai-modify-location/route.ts | /api/novel-promotion/{projectId}/ai-modify-location | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/ai-modify-shot-prompt/route.ts | /api/novel-promotion/{projectId}/ai-modify-shot-prompt | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/analyze-global/route.ts | /api/novel-promotion/{projectId}/analyze-global | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/analyze-shot-variants/route.ts | /api/novel-promotion/{projectId}/analyze-shot-variants | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/analyze/route.ts | /api/novel-promotion/{projectId}/analyze | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/assets/route.ts | /api/novel-promotion/{projectId}/assets | GET | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/character-profile/batch-confirm/route.ts | /api/novel-promotion/{projectId}/character-profile/batch-confirm | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/character-profile/confirm/route.ts | /api/novel-promotion/{projectId}/character-profile/confirm | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/character-voice/route.ts | /api/novel-promotion/{projectId}/character-voice | PATCH,POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/character/appearance/route.ts | /api/novel-promotion/{projectId}/character/appearance | DELETE,PATCH,POST | missing | 角色子形象管理（新增/修改描述/删除并重排 appearanceIndex）未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/character/confirm-selection/route.ts | /api/novel-promotion/{projectId}/character/confirm-selection | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/character/route.ts | /api/novel-promotion/{projectId}/character | DELETE,PATCH,POST | missing | 项目角色 CRUD（含创建后异步触发参考图/生成图）未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/cleanup-unselected-images/route.ts | /api/novel-promotion/{projectId}/cleanup-unselected-images | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/clips/[clipId]/route.ts | /api/novel-promotion/{projectId}/clips/{clipId} | PATCH | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/clips/route.ts | /api/novel-promotion/{projectId}/clips | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/copy-from-global/route.ts | /api/novel-promotion/{projectId}/copy-from-global | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/download-images/route.ts | /api/novel-promotion/{projectId}/download-images | GET | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/download-videos/route.ts | /api/novel-promotion/{projectId}/download-videos | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/download-voices/route.ts | /api/novel-promotion/{projectId}/download-voices | GET | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/editor/route.ts | /api/novel-promotion/{projectId}/editor | DELETE,GET,PUT | missing | 视频编辑器工程数据（videoEditorProject）读写删除未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/episodes/[episodeId]/route.ts | /api/novel-promotion/{projectId}/episodes/{episodeId} | DELETE,GET,PATCH | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/episodes/batch/route.ts | /api/novel-promotion/{projectId}/episodes/batch | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/episodes/route.ts | /api/novel-promotion/{projectId}/episodes | GET,POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/episodes/split-by-markers/route.ts | /api/novel-promotion/{projectId}/episodes/split-by-markers | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/episodes/split/route.ts | /api/novel-promotion/{projectId}/episodes/split | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/generate-character-image/route.ts | /api/novel-promotion/{projectId}/generate-character-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/generate-image/route.ts | /api/novel-promotion/{projectId}/generate-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/generate-video/route.ts | /api/novel-promotion/{projectId}/generate-video | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/insert-panel/route.ts | /api/novel-promotion/{projectId}/insert-panel | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/lip-sync/route.ts | /api/novel-promotion/{projectId}/lip-sync | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/location/confirm-selection/route.ts | /api/novel-promotion/{projectId}/location/confirm-selection | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/location/route.ts | /api/novel-promotion/{projectId}/location | DELETE,PATCH,POST | missing | 项目场景 CRUD（含 locationImage 首图初始化和更新）未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/modify-asset-image/route.ts | /api/novel-promotion/{projectId}/modify-asset-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/modify-storyboard-image/route.ts | /api/novel-promotion/{projectId}/modify-storyboard-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/panel-link/route.ts | /api/novel-promotion/{projectId}/panel-link | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/panel-variant/route.ts | /api/novel-promotion/{projectId}/panel-variant | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/panel/route.ts | /api/novel-promotion/{projectId}/panel | DELETE,PATCH,POST,PUT | missing | Panel 级别 CRUD 与完整字段 PUT 更新未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/panel/select-candidate/route.ts | /api/novel-promotion/{projectId}/panel/select-candidate | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/photography-plan/route.ts | /api/novel-promotion/{projectId}/photography-plan | PUT | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/reference-to-character/route.ts | /api/novel-promotion/{projectId}/reference-to-character | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/regenerate-group/route.ts | /api/novel-promotion/{projectId}/regenerate-group | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/regenerate-panel-image/route.ts | /api/novel-promotion/{projectId}/regenerate-panel-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/regenerate-single-image/route.ts | /api/novel-promotion/{projectId}/regenerate-single-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/regenerate-storyboard-text/route.ts | /api/novel-promotion/{projectId}/regenerate-storyboard-text | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/route.ts | /api/novel-promotion/{projectId} | GET,PATCH | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/screenplay-conversion/route.ts | /api/novel-promotion/{projectId}/screenplay-conversion | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/script-to-storyboard-stream/route.ts | /api/novel-promotion/{projectId}/script-to-storyboard-stream | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/select-character-image/route.ts | /api/novel-promotion/{projectId}/select-character-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/select-location-image/route.ts | /api/novel-promotion/{projectId}/select-location-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/speaker-voice/route.ts | /api/novel-promotion/{projectId}/speaker-voice | GET,PATCH | missing | 按 episode 维护 speakerVoices JSON（GET/PATCH）未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/story-to-script-stream/route.ts | /api/novel-promotion/{projectId}/story-to-script-stream | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/storyboard-group/route.ts | /api/novel-promotion/{projectId}/storyboard-group | DELETE,POST,PUT | missing | 分镜组（clip+storyboard+panel）增删改序未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/storyboards/route.ts | /api/novel-promotion/{projectId}/storyboards | GET,PATCH | missing | 按 episode 查询/清理 storyboard lastError 未迁移 |
| allyvideo/src/app/api/novel-promotion/[projectId]/undo-regenerate/route.ts | /api/novel-promotion/{projectId}/undo-regenerate | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/update-appearance/route.ts | /api/novel-promotion/{projectId}/update-appearance | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/update-asset-label/route.ts | /api/novel-promotion/{projectId}/update-asset-label | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/update-location/route.ts | /api/novel-promotion/{projectId}/update-location | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/update-prompt/route.ts | /api/novel-promotion/{projectId}/update-prompt | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/upload-asset-image/route.ts | /api/novel-promotion/{projectId}/upload-asset-image | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/video-proxy/route.ts | /api/novel-promotion/{projectId}/video-proxy | GET | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/video-urls/route.ts | /api/novel-promotion/{projectId}/video-urls | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/voice-analyze/route.ts | /api/novel-promotion/{projectId}/voice-analyze | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/voice-design/route.ts | /api/novel-promotion/{projectId}/voice-design | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/voice-generate/route.ts | /api/novel-promotion/{projectId}/voice-generate | POST | implemented | crates/server/src/routes/novel.rs |
| allyvideo/src/app/api/novel-promotion/[projectId]/voice-lines/route.ts | /api/novel-promotion/{projectId}/voice-lines | DELETE,GET,PATCH,POST | missing | 配音台词 CRUD + 批量 speaker 音色更新未迁移 |
| allyvideo/src/app/api/projects/[projectId]/assets/route.ts | /api/projects/{projectId}/assets | GET | implemented | crates/server/src/routes/projects.rs |
| allyvideo/src/app/api/projects/[projectId]/costs/route.ts | /api/projects/{projectId}/costs | GET | billing_excluded | 计费相关（按需求移除） |
| allyvideo/src/app/api/projects/[projectId]/data/route.ts | /api/projects/{projectId}/data | GET | implemented | crates/server/src/routes/projects.rs |
| allyvideo/src/app/api/projects/[projectId]/route.ts | /api/projects/{projectId} | DELETE,GET,PATCH | implemented | crates/server/src/routes/projects.rs |
| allyvideo/src/app/api/projects/route.ts | /api/projects | GET,POST | implemented | crates/server/src/routes/projects.rs |
| allyvideo/src/app/api/runs/[runId]/cancel/route.ts | /api/runs/{runId}/cancel | POST | implemented | crates/server/src/routes/runs.rs |
| allyvideo/src/app/api/runs/[runId]/events/route.ts | /api/runs/{runId}/events | GET | implemented | crates/server/src/routes/runs.rs |
| allyvideo/src/app/api/runs/[runId]/route.ts | /api/runs/{runId} | GET | implemented | crates/server/src/routes/runs.rs |
| allyvideo/src/app/api/runs/route.ts | /api/runs | GET,POST | implemented | crates/server/src/routes/runs.rs |
| allyvideo/src/app/api/sse/route.ts | /api/sse | GET | implemented | crates/server/src/routes/sse.rs |
| allyvideo/src/app/api/system/boot-id/route.ts | /api/system/boot-id | GET | implemented | crates/server/src/routes/system.rs |
| allyvideo/src/app/api/task-target-states/route.ts | /api/task-target-states | POST | implemented | crates/server/src/routes/tasks.rs |
| allyvideo/src/app/api/tasks/[taskId]/route.ts | /api/tasks/{taskId} | DELETE,GET | implemented | crates/server/src/routes/tasks.rs |
| allyvideo/src/app/api/tasks/dismiss/route.ts | /api/tasks/dismiss | POST | implemented | crates/server/src/routes/tasks.rs |
| allyvideo/src/app/api/tasks/route.ts | /api/tasks | GET | implemented | crates/server/src/routes/tasks.rs |
| allyvideo/src/app/api/user-preference/route.ts | /api/user-preference | GET,PATCH | implemented | crates/server/src/routes/mod.rs + crates/server/src/routes/user.rs |
| allyvideo/src/app/api/user/api-config/route.ts | /api/user/api-config | GET,PUT | implemented | crates/server/src/routes/mod.rs + crates/server/src/routes/user.rs |
| allyvideo/src/app/api/user/api-config/test-connection/route.ts | /api/user/api-config/test-connection | POST | implemented | crates/server/src/routes/mod.rs + crates/server/src/routes/user.rs |
| allyvideo/src/app/api/user/balance/route.ts | /api/user/balance | GET | billing_excluded | 计费相关（按需求移除） |
| allyvideo/src/app/api/user/costs/details/route.ts | /api/user/costs/details | GET | billing_excluded | 计费相关（按需求移除） |
| allyvideo/src/app/api/user/costs/route.ts | /api/user/costs | GET | billing_excluded | 计费相关（按需求移除） |
| allyvideo/src/app/api/user/models/route.ts | /api/user/models | GET | implemented | crates/server/src/routes/mod.rs + crates/server/src/routes/user.rs |
| allyvideo/src/app/api/user/transactions/route.ts | /api/user/transactions | GET | billing_excluded | 计费相关（按需求移除） |

## 2) 明确缺失的非计费 API（9 文件）

- allyvideo/src/app/api/novel-promotion/[projectId]/character/appearance/route.ts -> /api/novel-promotion/{projectId}/character/appearance (DELETE,PATCH,POST)
  - 角色子形象管理（新增/修改描述/删除并重排 appearanceIndex）未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/character/route.ts -> /api/novel-promotion/{projectId}/character (DELETE,PATCH,POST)
  - 项目角色 CRUD（含创建后异步触发参考图/生成图）未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/editor/route.ts -> /api/novel-promotion/{projectId}/editor (DELETE,GET,PUT)
  - 视频编辑器工程数据（videoEditorProject）读写删除未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/location/route.ts -> /api/novel-promotion/{projectId}/location (DELETE,PATCH,POST)
  - 项目场景 CRUD（含 locationImage 首图初始化和更新）未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/panel/route.ts -> /api/novel-promotion/{projectId}/panel (DELETE,PATCH,POST,PUT)
  - Panel 级别 CRUD 与完整字段 PUT 更新未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/speaker-voice/route.ts -> /api/novel-promotion/{projectId}/speaker-voice (GET,PATCH)
  - 按 episode 维护 speakerVoices JSON（GET/PATCH）未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/storyboard-group/route.ts -> /api/novel-promotion/{projectId}/storyboard-group (DELETE,POST,PUT)
  - 分镜组（clip+storyboard+panel）增删改序未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/storyboards/route.ts -> /api/novel-promotion/{projectId}/storyboards (GET,PATCH)
  - 按 episode 查询/清理 storyboard lastError 未迁移
- allyvideo/src/app/api/novel-promotion/[projectId]/voice-lines/route.ts -> /api/novel-promotion/{projectId}/voice-lines (DELETE,GET,PATCH,POST)
  - 配音台词 CRUD + 批量 speaker 音色更新未迁移

## 3) Worker 任务类型与处理器对照

- 任务类型对齐: TS `TASK_TYPE` 37 个，Rust `handlers::dispatch` 37 个，差异 0
- 结论: 非计费 task type 已全部覆盖；大量 TS helper 文件在 Rust 中按模块合并。

| 原始 worker handler 文件 | 状态 | Rust 对应 |
|---|---|---|
| allyvideo/src/lib/workers/handlers/analyze-global-parse.ts | merged | crates/worker/src/handlers/text/analyze_global.rs |
| allyvideo/src/lib/workers/handlers/analyze-global-persist.ts | merged | crates/worker/src/handlers/text/analyze_global.rs |
| allyvideo/src/lib/workers/handlers/analyze-global-prompt.ts | merged | crates/worker/src/handlers/text/analyze_global.rs |
| allyvideo/src/lib/workers/handlers/analyze-global.ts | implemented | crates/worker/src/handlers/text/analyze_global.rs |
| allyvideo/src/lib/workers/handlers/analyze-novel.ts | implemented | crates/worker/src/handlers/text/analyze_novel.rs |
| allyvideo/src/lib/workers/handlers/asset-hub-ai-design.ts | implemented | crates/worker/src/handlers/text/asset_hub_ai_design.rs |
| allyvideo/src/lib/workers/handlers/asset-hub-ai-modify.ts | implemented | crates/worker/src/handlers/text/asset_hub_ai_modify.rs |
| allyvideo/src/lib/workers/handlers/asset-hub-image-task-handler.ts | implemented | crates/worker/src/handlers/image/asset_hub_image.rs |
| allyvideo/src/lib/workers/handlers/asset-hub-modify-task-handler.ts | implemented | crates/worker/src/handlers/image/asset_hub_modify.rs |
| allyvideo/src/lib/workers/handlers/character-image-task-handler.ts | implemented | crates/worker/src/handlers/image/character.rs |
| allyvideo/src/lib/workers/handlers/character-profile-helpers.ts | merged | crates/worker/src/handlers/text/character_profile.rs |
| allyvideo/src/lib/workers/handlers/character-profile.ts | implemented | crates/worker/src/handlers/text/character_profile.rs |
| allyvideo/src/lib/workers/handlers/clips-build.ts | implemented | crates/worker/src/handlers/text/clips_build.rs |
| allyvideo/src/lib/workers/handlers/episode-split.ts | implemented | crates/worker/src/handlers/text/episode_split.rs |
| allyvideo/src/lib/workers/handlers/image-task-handler-shared.ts | merged | crates/worker/src/handlers/image/shared.rs |
| allyvideo/src/lib/workers/handlers/image-task-handlers-core.ts | merged | crates/worker/src/handlers/image/*.rs |
| allyvideo/src/lib/workers/handlers/image-task-handlers.ts | merged | crates/worker/src/handlers/image/*.rs |
| allyvideo/src/lib/workers/handlers/llm-proxy.ts | merged | crates/core/src/llm/mod.rs + crates/worker/src/handlers/text/shared.rs |
| allyvideo/src/lib/workers/handlers/llm-stream.ts | merged | crates/core/src/llm/mod.rs + crates/worker/src/handlers/text/shared.rs |
| allyvideo/src/lib/workers/handlers/location-image-task-handler.ts | implemented | crates/worker/src/handlers/image/location.rs |
| allyvideo/src/lib/workers/handlers/modify-asset-image-task-handler.ts | implemented | crates/worker/src/handlers/image/modify.rs |
| allyvideo/src/lib/workers/handlers/panel-image-task-handler.ts | implemented | crates/worker/src/handlers/image/panel.rs |
| allyvideo/src/lib/workers/handlers/panel-variant-task-handler.ts | implemented | crates/worker/src/handlers/image/variant.rs |
| allyvideo/src/lib/workers/handlers/reference-to-character-helpers.ts | merged | crates/worker/src/handlers/text/reference_to_character.rs |
| allyvideo/src/lib/workers/handlers/reference-to-character.ts | implemented | crates/worker/src/handlers/text/reference_to_character.rs |
| allyvideo/src/lib/workers/handlers/resolve-analysis-model.ts | merged | crates/worker/src/handlers/text/shared.rs |
| allyvideo/src/lib/workers/handlers/screenplay-convert-helpers.ts | merged | crates/worker/src/handlers/text/screenplay_convert.rs |
| allyvideo/src/lib/workers/handlers/screenplay-convert.ts | implemented | crates/worker/src/handlers/text/screenplay_convert.rs |
| allyvideo/src/lib/workers/handlers/script-to-storyboard-helpers.ts | merged | crates/worker/src/handlers/text/script_to_storyboard.rs |
| allyvideo/src/lib/workers/handlers/script-to-storyboard.ts | implemented | crates/worker/src/handlers/text/script_to_storyboard.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-persist.ts | merged | crates/worker/src/handlers/text/ai_modify_appearance.rs + ai_modify_location.rs + ai_modify_shot_prompt.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt-appearance.ts | merged | crates/worker/src/handlers/text/ai_modify_appearance.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt-location.ts | merged | crates/worker/src/handlers/text/ai_modify_location.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt-runtime.ts | merged | crates/worker/src/handlers/text/shared.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt-shot.ts | merged | crates/worker/src/handlers/text/ai_modify_shot_prompt.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt-utils.ts | merged | crates/worker/src/handlers/text/shared.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-prompt.ts | merged | crates/worker/src/handlers/text/ai_modify_*.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-tasks.ts | merged | crates/worker/src/handlers/text/ai_modify_*.rs + analyze_shot_variants.rs |
| allyvideo/src/lib/workers/handlers/shot-ai-variants.ts | implemented | crates/worker/src/handlers/text/analyze_shot_variants.rs |
| allyvideo/src/lib/workers/handlers/story-to-script-helpers.ts | merged | crates/worker/src/handlers/text/story_to_script.rs |
| allyvideo/src/lib/workers/handlers/story-to-script.ts | implemented | crates/worker/src/handlers/text/story_to_script.rs |
| allyvideo/src/lib/workers/handlers/voice-analyze-helpers.ts | merged | crates/worker/src/handlers/text/voice_analyze.rs |
| allyvideo/src/lib/workers/handlers/voice-analyze.ts | implemented | crates/worker/src/handlers/text/voice_analyze.rs |
| allyvideo/src/lib/workers/handlers/voice-design.ts | implemented | crates/worker/src/handlers/voice/voice_design.rs + asset_hub_voice_design.rs |

## 4) Watchdog 差异

- 原始 `allyvideo/scripts/watchdog.ts` 包含两类行为:
  - `recoverQueuedTasks`: 处理 `queued && enqueuedAt IS NULL` 的漏入队任务并重入队
  - `cleanupZombieProcessingTasks`: 处理心跳超时任务（重试或失败）
- Rust `crates/watchdog/src/main.rs` 当前只实现了“心跳超时 -> 标记 failed + 发布 task.failed”路径。
- 差距: 缺少 queued 漏入队恢复流程（与原始行为不一致）。

## 5) 核心库模块迁移结论（非计费）

- `prompt-i18n`: 已迁移并拆分到 `crates/core/src/prompt_i18n/*`
- `model-capabilities`: 已迁移到 `crates/core/src/capabilities/*`
- `run-runtime`: 已迁移到 `crates/core/src/runtime/*`
- `api-config`: 已迁移到 `crates/core/src/api_config/mod.rs` + `crates/server/src/routes/user.rs`
- `llm`: 已在 `crates/core/src/llm/mod.rs` 合并实现（provider 级文件改为 Rust 内聚）
- `media`: 已有基础迁移（下载/归一化/上传），但目前 `upload_bytes_to_storage` 仅支持 `STORAGE_TYPE=local`，与原仓 COS 场景相比能力偏弱。
- 认证: 旧的 NextAuth 路由未保留，Rust 改为 JWT register/login/refresh。
