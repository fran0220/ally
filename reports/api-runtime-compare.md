# API Runtime Compare Report

- Generated at: 2026-03-04T08:30:28.682Z
- Total: 39
- Executed: 29
- Skipped: 10
- Pass: 5
- Fail: 24

| Case | Method | Path | Result | Legacy Status | Rust Status | Missing In Rust | Extra In Rust |
|---|---|---|---|---|---|---|---|
| system-healthz | GET | /healthz | FAIL | 404 | 200 |  |  |
| system-boot-id | GET | /api/system/boot-id | PASS | 200 | 200 |  |  |
| auth-register-invalid | POST | /api/auth/register | PASS | 400 | 400 |  |  |
| auth-login-invalid | POST | /api/auth/login | FAIL | 400 | 400 |  |  |
| auth-refresh-unauthorized | POST | /api/auth/refresh | FAIL | 400 | 401 |  |  |
| user-models-auth | GET | /api/user/models | FAIL | 401 | 200 | code, error, message, success | audio, image, lipsync, llm, video |
| user-api-config-auth | GET | /api/user/api-config | FAIL | 401 | 200 | code, error, message, success | capabilityDefaults, defaultModels, models, providers |
| user-api-config-test-connection-invalid | POST | /api/user/api-config/test-connection | FAIL | 401 | 401 |  | requestId |
| user-preference-auth | GET | /api/user-preference | FAIL | 401 | 200 | code, error, message, success | preference |
| projects-list-auth | GET | /api/projects?page=1&pageSize=5 | FAIL | 401 | 200 | code, error, message, success | pagination, projects |
| projects-create-invalid | POST | /api/projects | FAIL | 401 | 400 |  | requestId |
| tasks-list-auth | GET | /api/tasks?limit=5 | FAIL | 401 | 200 | code, error, message, success | tasks |
| tasks-dismiss-invalid | POST | /api/tasks/dismiss | FAIL | 401 | 400 |  | requestId |
| task-target-states-invalid | POST | /api/task-target-states | PASS | 400 | 400 |  |  |
| runs-list-auth | GET | /api/runs?limit=5 | FAIL | 401 | 200 | code, error, message, success | runs |
| runs-create-invalid | POST | /api/runs | FAIL | 401 | 400 |  | requestId |
| sse-unauthorized | GET | /api/sse?projectId=global-asset-hub | FAIL | 401 | 401 |  | requestId |
| asset-hub-folders-auth | GET | /api/asset-hub/folders | FAIL | 401 | 200 | code, error, message, success | folders |
| asset-hub-characters-auth | GET | /api/asset-hub/characters | FAIL | 401 | 200 | code, error, message, success | characters |
| asset-hub-picker-auth | GET | /api/asset-hub/picker?type=character | FAIL | 401 | 200 | code, error, message, success | characters |
| asset-hub-generate-image-submit | POST | /api/asset-hub/generate-image | FAIL | 401 | 400 |  | requestId |
| asset-hub-ai-design-character-submit | POST | /api/asset-hub/ai-design-character | FAIL | 401 | 400 |  | requestId |
| asset-hub-voice-design-submit | POST | /api/asset-hub/voice-design | FAIL | 401 | 400 |  | requestId |
| novel-root-unauthorized | GET | /api/novel-promotion/runtime-compare-project | FAIL | 401 | 401 |  | requestId |
| admin-ai-config-unauthorized | GET | /api/admin/ai-config | FAIL | 404 | 401 |  |  |
| cos-image-unauthorized | GET | /api/cos/image?key=runtime-compare.png | FAIL | 500 | 404 | category, code, message, requestId, retryable, success, userMessageKey |  |
| cos-image-invalid-key-auth | GET | /api/cos/image?key= | PASS | 400 | 400 |  |  |
| files-not-found | GET | /api/files/runtime-compare-not-found.png | PASS | 404 | 404 |  |  |
| media-not-found | GET | /m/runtime-compare-missing | FAIL | 500 | 404 |  |  |

## Skipped Cases

- projects-get-by-id (GET /api/projects/{{WW_PROJECT_ID}}) missing env: WW_PROJECT_ID
- projects-assets-by-id (GET /api/projects/{{WW_PROJECT_ID}}/assets) missing env: WW_PROJECT_ID
- projects-data-by-id (GET /api/projects/{{WW_PROJECT_ID}}/data) missing env: WW_PROJECT_ID
- tasks-get-by-id (GET /api/tasks/{{WW_TASK_ID}}?includeEvents=1&eventsLimit=50) missing env: WW_TASK_ID
- runs-get-by-id (GET /api/runs/{{WW_RUN_ID}}) missing env: WW_RUN_ID
- runs-events-by-id (GET /api/runs/{{WW_RUN_ID}}/events?afterSeq=0&limit=50) missing env: WW_RUN_ID
- novel-root-by-id (GET /api/novel-promotion/{{WW_PROJECT_ID}}) missing env: WW_PROJECT_ID
- novel-assets-by-id (GET /api/novel-promotion/{{WW_PROJECT_ID}}/assets) missing env: WW_PROJECT_ID
- novel-generate-image-submit (POST /api/novel-promotion/{{WW_PROJECT_ID}}/generate-image) missing env: WW_PROJECT_ID
- novel-generate-video-submit (POST /api/novel-promotion/{{WW_PROJECT_ID}}/generate-video) missing env: WW_PROJECT_ID

## Failures

- system-healthz (GET /healthz)
  - legacy status 404 != expected 200
  - status mismatch legacy=404 rust=200
  - legacy response is not a JSON object
- auth-login-invalid (POST /api/auth/login)
  - legacy response is not a JSON object
- auth-refresh-unauthorized (POST /api/auth/refresh)
  - legacy status 400 != expected 401
  - status mismatch legacy=400 rust=401
  - legacy response is not a JSON object
- user-models-auth (GET /api/user/models)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[audio, image, lipsync, llm, video]
- user-api-config-auth (GET /api/user/api-config)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[capabilityDefaults, defaultModels, models, providers]
- user-api-config-test-connection-invalid (POST /api/user/api-config/test-connection)
  - legacy status 401 != expected 400
  - rust status 401 != expected 400
  - top-level key mismatch missing=[] extra=[requestId]
- user-preference-auth (GET /api/user-preference)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[preference]
- projects-list-auth (GET /api/projects?page=1&pageSize=5)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[pagination, projects]
- projects-create-invalid (POST /api/projects)
  - legacy status 401 != expected 400
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- tasks-list-auth (GET /api/tasks?limit=5)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[tasks]
- tasks-dismiss-invalid (POST /api/tasks/dismiss)
  - legacy status 401 != expected 400
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- runs-list-auth (GET /api/runs?limit=5)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[runs]
- runs-create-invalid (POST /api/runs)
  - legacy status 401 != expected 400
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- sse-unauthorized (GET /api/sse?projectId=global-asset-hub)
  - top-level key mismatch missing=[] extra=[requestId]
- asset-hub-folders-auth (GET /api/asset-hub/folders)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[folders]
- asset-hub-characters-auth (GET /api/asset-hub/characters)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[characters]
- asset-hub-picker-auth (GET /api/asset-hub/picker?type=character)
  - legacy status 401 != expected 200
  - status mismatch legacy=401 rust=200
  - top-level key mismatch missing=[code, error, message, success] extra=[characters]
- asset-hub-generate-image-submit (POST /api/asset-hub/generate-image)
  - legacy status 401 != expected 200
  - rust status 400 != expected 200
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- asset-hub-ai-design-character-submit (POST /api/asset-hub/ai-design-character)
  - legacy status 401 != expected 200
  - rust status 400 != expected 200
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- asset-hub-voice-design-submit (POST /api/asset-hub/voice-design)
  - legacy status 401 != expected 200
  - rust status 400 != expected 200
  - status mismatch legacy=401 rust=400
  - top-level key mismatch missing=[] extra=[requestId]
- novel-root-unauthorized (GET /api/novel-promotion/runtime-compare-project)
  - top-level key mismatch missing=[] extra=[requestId]
- admin-ai-config-unauthorized (GET /api/admin/ai-config)
  - legacy status 404 != expected 401
  - status mismatch legacy=404 rust=401
  - legacy response is not a JSON object
- cos-image-unauthorized (GET /api/cos/image?key=runtime-compare.png)
  - legacy status 500 != expected 401
  - rust status 404 != expected 401
  - status mismatch legacy=500 rust=404
  - top-level key mismatch missing=[category, code, message, requestId, retryable, success, userMessageKey] extra=[]
- media-not-found (GET /m/runtime-compare-missing)
  - legacy status 500 != expected 404
  - status mismatch legacy=500 rust=404
  - legacy response is not a JSON object

