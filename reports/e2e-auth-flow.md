# E2E Auth Flow Report

- Generated at: 2026-03-04T10:43:51.231Z
- Base URL: http://185.200.65.233/
- Test user: e2e-auth-1772621025745@test.com
- Total steps: 18
- Pass: 18
- Fail: 0

| Step | Method | Path | Result | Expected | Actual | Detail |
|---|---|---|---|---|---|---|
| auth-register | POST | /api/auth/register | PASS | 200 or 201 | 201 | response.refreshToken missing; refresh step will reuse access token as request body value |
| protected-user-preference-with-token | GET | /api/user-preference | PASS | 200 | 200 | {"preference":{"analysisModel":null,"artStyle":"american-comic","capabilityDefaults":null,"characterModel":null,"createdAt":"2026-03-04T10:43:47.024","editModel":null,"id":"62d0293b-9cb6-4716-94dc-b23a8b6e6943","lipSyncModel":null,"locationModel":null,"storyboardModel":null,"ttsRate":"+50%","updatedAt":"2026-03-04T1... |
| protected-user-models-with-token | GET | /api/user/models | PASS | 200 | 200 | {"llm":[{"value":"openai-compatible::glm-5","label":"GLM-5","provider":"openai-compatible","providerName":"LLM Proxy"}],"image":[{"value":"fal::banana-2","label":"Banana 2","provider":"fal","providerName":"fal.ai"}],"video":[{"value":"fal::fal-wan25","label":"Wan 2.5","provider":"fal","providerName":"fal.ai"},{"valu... |
| protected-user-api-config-with-token | GET | /api/user/api-config | PASS | 200 | 200 | {"capabilityDefaults":{},"defaultModels":{"analysisModel":"","characterModel":"","editModel":"","lipSyncModel":"","locationModel":"","storyboardModel":"","videoModel":""},"models":[{"enabled":true,"modelId":"glm-5","modelKey":"openai-compatible::glm-5","name":"GLM-5","price":0,"provider":"openai-compatible","type":"... |
| protected-projects-list-with-token | GET | /api/projects?page=1&pageSize=5 | PASS | 200 | 200 | {"pagination":{"page":1,"pageSize":5,"total":0,"totalPages":0},"projects":[]} |
| protected-asset-hub-folders-with-token | GET | /api/asset-hub/folders | PASS | 200 | 200 | {"folders":[]} |
| protected-user-preference-without-token | GET | /api/user-preference | PASS | 401 | 401 | {"success":false,"requestId":"5003b1e2-096b-4c4b-8b8e-8aea2790fd99","error":{"code":"UNAUTHORIZED","message":"missing auth token","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"missing auth token"} |
| protected-user-models-without-token | GET | /api/user/models | PASS | 401 | 401 | {"success":false,"requestId":"00112782-8a5c-40b1-b163-3fde4287c9cb","error":{"code":"UNAUTHORIZED","message":"missing auth token","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"missing auth token"} |
| protected-user-api-config-without-token | GET | /api/user/api-config | PASS | 401 | 401 | {"success":false,"requestId":"21412256-3667-48f6-ab50-a5b39b0be66f","error":{"code":"UNAUTHORIZED","message":"missing auth token","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"missing auth token"} |
| protected-projects-list-without-token | GET | /api/projects?page=1&pageSize=5 | PASS | 401 | 401 | {"success":false,"requestId":"eb8a81d8-f421-44b4-8914-943fa3b0eb98","error":{"code":"UNAUTHORIZED","message":"missing auth token","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"missing auth token"} |
| protected-asset-hub-folders-without-token | GET | /api/asset-hub/folders | PASS | 401 | 401 | {"success":false,"requestId":"9aa2ce7b-d5a9-4e05-85a1-a534decad8b5","error":{"code":"UNAUTHORIZED","message":"missing auth token","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"missing auth token"} |
| protected-user-preference-invalid-token | GET | /api/user-preference | PASS | 401 | 401 | {"success":false,"requestId":"97ccdae3-86b0-4eda-a408-8b1e0a395dd2","error":{"code":"UNAUTHORIZED","message":"invalid token: InvalidToken","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"invalid token: InvalidToken"} |
| protected-user-models-invalid-token | GET | /api/user/models | PASS | 401 | 401 | {"success":false,"requestId":"ae842838-3e70-4c40-8290-91662e56bdef","error":{"code":"UNAUTHORIZED","message":"invalid token: InvalidToken","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"invalid token: InvalidToken"} |
| protected-user-api-config-invalid-token | GET | /api/user/api-config | PASS | 401 | 401 | {"success":false,"requestId":"37f8adce-c54b-460f-887a-5606b68bbaf1","error":{"code":"UNAUTHORIZED","message":"invalid token: InvalidToken","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"invalid token: InvalidToken"} |
| protected-projects-list-invalid-token | GET | /api/projects?page=1&pageSize=5 | PASS | 401 | 401 | {"success":false,"requestId":"f338168f-a4f3-4c92-b57e-efddb46e735d","error":{"code":"UNAUTHORIZED","message":"invalid token: InvalidToken","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"invalid token: InvalidToken"} |
| protected-asset-hub-folders-invalid-token | GET | /api/asset-hub/folders | PASS | 401 | 401 | {"success":false,"requestId":"d87f7849-3b84-42fc-849f-5ad8bf804c5e","error":{"code":"UNAUTHORIZED","message":"invalid token: InvalidToken","retryable":false,"category":"AUTH","user_message_key":"errors.UNAUTHORIZED"},"code":"UNAUTHORIZED","message":"invalid token: InvalidToken"} |
| auth-refresh | POST | /api/auth/refresh | PASS | 200 | 200 | {"token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmYjIwZjVhYi0wMzljLTQyNDktOGM4Yi1jOWJkMTIyNWQwNWIiLCJ1c2VybmFtZSI6ImUyZS1hdXRoLTE3NzI2MjEwMjU3NDVAdGVzdC5jb20iLCJyb2xlIjoidXNlciIsImlhdCI6MTc3MjYyMTAzMCwiZXhwIjoxNzczMjI1ODMwfQ.pe-hD-JxDsZk8JuRMCQ0-2fdN6ASa6LfvQSOAmwbu-U","user":{"id":"fb20f5ab-039c-4249-8c8b-... |
| auth-login | POST | /api/auth/login | PASS | 200 | 200 | {"token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmYjIwZjVhYi0wMzljLTQyNDktOGM4Yi1jOWJkMTIyNWQwNWIiLCJ1c2VybmFtZSI6ImUyZS1hdXRoLTE3NzI2MjEwMjU3NDVAdGVzdC5jb20iLCJyb2xlIjoidXNlciIsImlhdCI6MTc3MjYyMTAzMSwiZXhwIjoxNzczMjI1ODMxfQ.YigMsMlk_Hke7K9bIfLHV4AMqjs0i8_65a3i0BKzJAU","user":{"id":"fb20f5ab-039c-4249-8c8b-... |

## Failures

- none

## Warnings

- auth-register (POST /api/auth/register)
  - response.refreshToken missing; refresh step will reuse access token as request body value

