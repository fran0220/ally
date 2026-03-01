# API Runtime Compare Report

- Generated at: 2026-03-01T13:42:46.537Z
- Total: 39
- Executed: 39
- Skipped: 0
- Pass: 17
- Fail: 22

| Case | Method | Path | Result | Legacy Status | Rust Status | Missing In Rust | Extra In Rust |
|---|---|---|---|---|---|---|---|
| system-healthz | GET | /healthz | FAIL | 404 | 200 |  |  |
| system-boot-id | GET | /api/system/boot-id | PASS | 200 | 200 |  |  |
| auth-register-invalid | POST | /api/auth/register | FAIL | 400 | 400 | requestId |  |
| auth-login-invalid | POST | /api/auth/login | FAIL | 400 | 400 |  |  |
| auth-refresh-unauthorized | POST | /api/auth/refresh | FAIL | 400 | 401 |  |  |
| user-models-auth | GET | /api/user/models | PASS | 200 | 200 |  |  |
| user-api-config-auth | GET | /api/user/api-config | PASS | 200 | 200 |  |  |
| user-api-config-test-connection-invalid | POST | /api/user/api-config/test-connection | FAIL | 401 | 400 |  |  |
| user-preference-auth | GET | /api/user-preference | PASS | 200 | 200 |  |  |
| projects-list-auth | GET | /api/projects?page=1&pageSize=5 | PASS | 200 | 200 |  |  |
| projects-create-invalid | POST | /api/projects | FAIL | 400 | 400 | requestId |  |
| projects-get-by-id | GET | /api/projects/82359bde-da63-4da6-af6b-19c33114f5c3 | PASS | 200 | 200 |  |  |
| projects-assets-by-id | GET | /api/projects/82359bde-da63-4da6-af6b-19c33114f5c3/assets | PASS | 200 | 200 |  |  |
| projects-data-by-id | GET | /api/projects/82359bde-da63-4da6-af6b-19c33114f5c3/data | PASS | 200 | 200 |  |  |
| tasks-list-auth | GET | /api/tasks?limit=5 | PASS | 200 | 200 |  |  |
| tasks-dismiss-invalid | POST | /api/tasks/dismiss | FAIL | 400 | 400 | requestId |  |
| task-target-states-invalid | POST | /api/task-target-states | FAIL | 400 | 400 | requestId |  |
| tasks-get-by-id | GET | /api/tasks/d3e3b5e5-ed29-4f7a-a86d-0d6febfec7cd?includeEvents=1&eventsLimit=50 | PASS | 200 | 200 |  |  |
| runs-list-auth | GET | /api/runs?limit=5 | FAIL | 500 | 200 | category, code, error, message, requestId, retryable, success, userMessageKey | runs |
| runs-create-invalid | POST | /api/runs | FAIL | 400 | 400 | requestId |  |
| runs-get-by-id | GET | /api/runs/2ed815b9-6a97-42a6-a415-2b7627c80d8c | FAIL | 500 | 200 | category, code, error, message, requestId, retryable, success, userMessageKey | run, steps |
| runs-events-by-id | GET | /api/runs/2ed815b9-6a97-42a6-a415-2b7627c80d8c/events?afterSeq=0&limit=50 | FAIL | 500 | 200 | category, code, error, message, requestId, retryable, success, userMessageKey | afterSeq, events, runId |
| sse-unauthorized | GET | /api/sse?projectId=global-asset-hub | PASS | 401 | 401 |  |  |
| asset-hub-folders-auth | GET | /api/asset-hub/folders | PASS | 200 | 200 |  |  |
| asset-hub-characters-auth | GET | /api/asset-hub/characters | PASS | 200 | 200 |  |  |
| asset-hub-picker-auth | GET | /api/asset-hub/picker?type=character | PASS | 200 | 200 |  |  |
| asset-hub-generate-image-submit | POST | /api/asset-hub/generate-image | FAIL | 400 | 200 | code, error, field, message, requestId | async, deduped, status, taskId |
| asset-hub-ai-design-character-submit | POST | /api/asset-hub/ai-design-character | FAIL | 400 | 200 | code, error, message, requestId | async, deduped, status, taskId |
| asset-hub-voice-design-submit | POST | /api/asset-hub/voice-design | FAIL | 400 | 200 | code, error, field, message, requestId | async, deduped, status, taskId |
| novel-root-unauthorized | GET | /api/novel-promotion/runtime-compare-project | PASS | 401 | 401 |  |  |
| novel-root-by-id | GET | /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3 | FAIL | 200 | 200 |  | project |
| novel-assets-by-id | GET | /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3/assets | PASS | 200 | 200 |  |  |
| novel-generate-image-submit | POST | /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3/generate-image | FAIL | 400 | 200 | code, error, field, message, requestId | async, deduped, status, taskId |
| novel-generate-video-submit | POST | /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3/generate-video | FAIL | 400 | 200 | code, error, field, message, requestId | async, deduped, status, taskId, tasks, total |
| admin-ai-config-unauthorized | GET | /api/admin/ai-config | PASS | 401 | 401 |  |  |
| cos-image-unauthorized | GET | /api/cos/image?key=runtime-compare.png | FAIL | 404 | 401 |  | code, message, success |
| cos-image-invalid-key-auth | GET | /api/cos/image?key= | FAIL | 400 | 400 | requestId |  |
| files-not-found | GET | /api/files/runtime-compare-not-found.png | FAIL | 404 | 404 |  | code, message, success |
| media-not-found | GET | /m/runtime-compare-missing | FAIL | 404 | 404 |  | code, message, success |

## Skipped Cases

- none

## Failures

- system-healthz (GET /healthz)
  - legacy status 404 != expected 200
  - status mismatch legacy=404 rust=200
  - legacy response is not a JSON object
- auth-register-invalid (POST /api/auth/register)
  - top-level key mismatch missing=[requestId] extra=[]
- auth-login-invalid (POST /api/auth/login)
  - legacy response is not a JSON object
- auth-refresh-unauthorized (POST /api/auth/refresh)
  - legacy status 400 != expected 401
  - status mismatch legacy=400 rust=401
  - legacy response is not a JSON object
- user-api-config-test-connection-invalid (POST /api/user/api-config/test-connection)
  - legacy status 401 != expected 400
  - status mismatch legacy=401 rust=400
- projects-create-invalid (POST /api/projects)
  - top-level key mismatch missing=[requestId] extra=[]
- tasks-dismiss-invalid (POST /api/tasks/dismiss)
  - top-level key mismatch missing=[requestId] extra=[]
- task-target-states-invalid (POST /api/task-target-states)
  - top-level key mismatch missing=[requestId] extra=[]
- runs-list-auth (GET /api/runs?limit=5)
  - legacy status 500 != expected 200
  - status mismatch legacy=500 rust=200
  - top-level key mismatch missing=[category, code, error, message, requestId, retryable, success, userMessageKey] extra=[runs]
- runs-create-invalid (POST /api/runs)
  - top-level key mismatch missing=[requestId] extra=[]
- runs-get-by-id (GET /api/runs/2ed815b9-6a97-42a6-a415-2b7627c80d8c)
  - legacy status 500 != expected 200
  - status mismatch legacy=500 rust=200
  - top-level key mismatch missing=[category, code, error, message, requestId, retryable, success, userMessageKey] extra=[run, steps]
- runs-events-by-id (GET /api/runs/2ed815b9-6a97-42a6-a415-2b7627c80d8c/events?afterSeq=0&limit=50)
  - legacy status 500 != expected 200
  - status mismatch legacy=500 rust=200
  - top-level key mismatch missing=[category, code, error, message, requestId, retryable, success, userMessageKey] extra=[afterSeq, events, runId]
- asset-hub-generate-image-submit (POST /api/asset-hub/generate-image)
  - legacy status 400 != expected 200
  - status mismatch legacy=400 rust=200
  - top-level key mismatch missing=[code, error, field, message, requestId] extra=[async, deduped, status, taskId]
- asset-hub-ai-design-character-submit (POST /api/asset-hub/ai-design-character)
  - legacy status 400 != expected 200
  - status mismatch legacy=400 rust=200
  - top-level key mismatch missing=[code, error, message, requestId] extra=[async, deduped, status, taskId]
- asset-hub-voice-design-submit (POST /api/asset-hub/voice-design)
  - legacy status 400 != expected 200
  - status mismatch legacy=400 rust=200
  - top-level key mismatch missing=[code, error, field, message, requestId] extra=[async, deduped, status, taskId]
- novel-root-by-id (GET /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3)
  - top-level key mismatch missing=[] extra=[project]
- novel-generate-image-submit (POST /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3/generate-image)
  - legacy status 400 != expected 200
  - status mismatch legacy=400 rust=200
  - top-level key mismatch missing=[code, error, field, message, requestId] extra=[async, deduped, status, taskId]
- novel-generate-video-submit (POST /api/novel-promotion/82359bde-da63-4da6-af6b-19c33114f5c3/generate-video)
  - legacy status 400 != expected 200
  - status mismatch legacy=400 rust=200
  - top-level key mismatch missing=[code, error, field, message, requestId] extra=[async, deduped, status, taskId, tasks, total]
- cos-image-unauthorized (GET /api/cos/image?key=runtime-compare.png)
  - legacy status 404 != expected 401
  - status mismatch legacy=404 rust=401
  - top-level key mismatch missing=[] extra=[code, message, success]
- cos-image-invalid-key-auth (GET /api/cos/image?key=)
  - top-level key mismatch missing=[requestId] extra=[]
- files-not-found (GET /api/files/runtime-compare-not-found.png)
  - top-level key mismatch missing=[] extra=[code, message, success]
- media-not-found (GET /m/runtime-compare-missing)
  - top-level key mismatch missing=[] extra=[code, message, success]

