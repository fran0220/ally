# E2E Project Lifecycle Report

- Generated at: 2026-03-04T10:43:41.505Z
- Base URL: http://185.200.65.233/
- E2E user: e2e-project-lifecycle-1772621017608-20657a5b@test.com
- Project ID: 1d8bfc22-93f8-491a-ac3e-eee0cecbe548
- Total: 12
- Pass: 12
- Fail: 0

| Step | Result | Method | Path | HTTP Status | Duration(ms) | Notes |
|---|---|---|---|---|---|---|
| auth-register | PASS | POST | /api/auth/register | 201 | 710 | registered e2e-project-lifecycle-1772621017608-20657a5b@test.com |
| create-project-empty-name | PASS | POST | /api/projects | 400 | 646 | empty name rejected |
| create-project | PASS | POST | /api/projects | 200 | 258 | projectId=1d8bfc22-93f8-491a-ac3e-eee0cecbe548 |
| list-projects | PASS | GET | /api/projects?page=1&pageSize=20 | 200 | 251 | listed=1 |
| search-projects | PASS | GET | /api/projects?page=1&pageSize=5&search=E2E | 200 | 258 | matches=1 |
| get-project | PASS | GET | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548 | 200 | 253 | name=E2E Test Project 1772621017608 |
| get-project-data | PASS | GET | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548/data | 200 | 257 | novelPromotionData.id=1fffc706-49a4-49fc-be04-582efb1a02c4 |
| get-project-assets | PASS | GET | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548/assets | 200 | 252 | characters=0, locations=0 |
| update-project | PASS | PATCH | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548 | 200 | 260 | name updated to Updated E2E |
| get-project-not-found | PASS | GET | /api/projects/80bb9e36-bebd-4980-a896-fe0c32f2dd08 | 404 | 248 | missing project 80bb9e36-bebd-4980-a896-fe0c32f2dd08 returns 404 |
| delete-project | PASS | DELETE | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548 | 200 | 255 | cosFilesDeleted=0 cosFilesFailed=0 |
| verify-project-deleted | PASS | GET | /api/projects/1d8bfc22-93f8-491a-ac3e-eee0cecbe548 | 404 | 248 | project 1d8bfc22-93f8-491a-ac3e-eee0cecbe548 no longer accessible |

## Failures

- none

