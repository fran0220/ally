# E2E Run All Summary

- Generated at: 2026-03-04T10:43:25.720Z
- Base URL: http://185.200.65.233/
- Clean requested: true
- E2E user: e2e-run-all-1772621005720-68acb41c@test.local
- User ID: dee000ba-64a0-465e-8181-21edd3f34d0a
- Project ID: b07beac8-1fe7-4139-8759-b0ca95329a71
- Phases: 6 (pass=6, fail=0)
- Steps: 32 (pass=32, fail=0)
- Cleanup steps: 4 (pass=4, fail=0)

## Phases

| Phase | Result | Steps | Pass | Fail | Duration(ms) | Error |
|---|---|---|---|---|---|---|
| auth-flow | PASS | 3 | 3 | 0 | 1740 |  |
| project-lifecycle | PASS | 5 | 5 | 0 | 1303 |  |
| asset-hub-crud | PASS | 7 | 7 | 0 | 1842 |  |
| novel-promotion | PASS | 7 | 7 | 0 | 1845 |  |
| user-config | PASS | 5 | 5 | 0 | 1313 |  |
| admin-billing | PASS | 5 | 5 | 0 | 1299 |  |

## auth-flow

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| register-user | PASS | POST | /api/auth/register | 201 | 759 | userId=dee000ba-64a0-465e-8181-21edd3f34d0a |
| login-user | PASS | POST | /api/auth/login | 200 | 723 | role=user |
| refresh-token | PASS | POST | /api/auth/refresh | 200 | 258 | token refreshed |

## project-lifecycle

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| create-project | PASS | POST | /api/projects | 200 | 265 | projectId=b07beac8-1fe7-4139-8759-b0ca95329a71 |
| list-projects | PASS | GET | /api/projects?page=1&pageSize=20 | 200 | 258 | listed=1 |
| get-project | PASS | GET | /api/projects/b07beac8-1fe7-4139-8759-b0ca95329a71 | 200 | 259 | name=E2E Run All 1772621005720 |
| get-project-data | PASS | GET | /api/projects/b07beac8-1fe7-4139-8759-b0ca95329a71/data | 200 | 263 | novelId=868e5a44-2500-46ad-b28b-bb90f80caf1d |
| get-project-assets | PASS | GET | /api/projects/b07beac8-1fe7-4139-8759-b0ca95329a71/assets | 200 | 258 | characters=0, locations=0 |

## asset-hub-crud

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| create-asset-character | PASS | POST | /api/asset-hub/characters | 200 | 267 | characterId=4157d2d4-da8f-40f2-977d-38314cd6c1e1 |
| update-asset-character | PASS | PATCH | /api/asset-hub/characters/4157d2d4-da8f-40f2-977d-38314cd6c1e1 | 200 | 266 | updated=4157d2d4-da8f-40f2-977d-38314cd6c1e1 |
| create-asset-location | PASS | POST | /api/asset-hub/locations | 200 | 265 | locationId=cea05a69-24d9-4199-93ec-e93734c47d9b |
| create-asset-voice | PASS | POST | /api/asset-hub/voices | 200 | 262 | voiceId=2559876e-415f-42bc-b059-d1d4c633a332 |
| list-asset-locations | PASS | GET | /api/asset-hub/locations | 200 | 260 | count=1 include-new |
| list-asset-voices | PASS | GET | /api/asset-hub/voices | 200 | 259 | count=1 include-new |
| delete-asset-character | PASS | DELETE | /api/asset-hub/characters/4157d2d4-da8f-40f2-977d-38314cd6c1e1 | 200 | 262 | deleted |

## novel-promotion

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| get-novel-root | PASS | GET | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71 | 200 | 262 | episodes=0 |
| get-novel-episodes | PASS | GET | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/episodes | 200 | 263 | episodes=0 |
| create-novel-episode | PASS | POST | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/episodes | 200 | 268 | episodeId=1541e4ca-c195-4d1a-8f56-c91c3227fbad |
| get-novel-characters | PASS | GET | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/characters | 200 | 259 | characters=0 |
| get-novel-locations | PASS | GET | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/locations | 200 | 262 | locations=0 |
| create-novel-character | PASS | POST | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/characters | 200 | 269 | characterId=47dc8e9d-b7ca-484f-a8ca-55fd859a818c |
| get-novel-storyboards | PASS | GET | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/storyboards?episodeId=1541e4ca-c195-4d1a-8f56-c91c3227fbad | 200 | 262 | storyboards=0 |

## user-config

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| get-user-models | PASS | GET | /api/user/models | 200 | 258 | llm=1 |
| get-user-api-config | PASS | GET | /api/user/api-config | 200 | 267 | providers=3, models=9 |
| get-user-preference | PASS | GET | /api/user-preference | 200 | 265 | artStyle=american-comic |
| patch-user-preference | PASS | PATCH | /api/user-preference | 200 | 266 | artStyle=american-comic |
| test-user-api-config-invalid | PASS | POST | /api/user/api-config/test-connection | 400 | 257 | empty payload rejected as expected |

## admin-billing

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| admin-ai-config-non-admin | PASS | GET | /api/admin/ai-config | 403 | 256 | non-admin rejected as expected |
| billing-balance | PASS | GET | /api/user/balance | 200 | 263 | currency=CNY |
| billing-costs | PASS | GET | /api/user/costs | 200 | 262 | projects=0 |
| billing-cost-details | PASS | GET | /api/user/costs/details?page=1&pageSize=10 | 200 | 258 | records=0 |
| billing-transactions | PASS | GET | /api/user/transactions?page=1&pageSize=10 | 200 | 259 | transactions=0 |

## Cleanup

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |
|---|---|---|---|---|---|---|
| delete-novel-character | PASS | DELETE | /api/novel-promotion/b07beac8-1fe7-4139-8759-b0ca95329a71/characters | 200 | 263 | characterId=47dc8e9d-b7ca-484f-a8ca-55fd859a818c |
| delete-asset-location-cea05a69-24d9-4199-93ec-e93734c47d9b | PASS | DELETE | /api/asset-hub/locations/cea05a69-24d9-4199-93ec-e93734c47d9b | 200 | 261 | done |
| delete-asset-voice-2559876e-415f-42bc-b059-d1d4c633a332 | PASS | DELETE | /api/asset-hub/voices/2559876e-415f-42bc-b059-d1d4c633a332 | 200 | 1689 | done |
| delete-project | PASS | DELETE | /api/projects/b07beac8-1fe7-4139-8759-b0ca95329a71 | 200 | 263 | projectId=b07beac8-1fe7-4139-8759-b0ca95329a71 |

## Failures

- none

