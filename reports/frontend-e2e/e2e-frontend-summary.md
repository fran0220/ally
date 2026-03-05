# Frontend E2E Summary

- Generated at: 2026-03-04T16:27:52.936Z
- Base URL: http://127.0.0.1:9/
- Headless: true
- Timeout(ms): 1500
- Clean requested: false
- E2E user: e2e-frontend-1772641672936-e3ecd00f@test.local
- Project ID: N/A
- Phases: 8 (pass=0, fail=8)
- Steps: 8 (pass=0, fail=8)
- Cleanup steps: 0 (pass=0, fail=0)
- Diagnostics: console.error=0, console.warning=0, pageerror=0, requestfailed=7
- Screenshots: 8

## Phases

| Phase | Result | Steps | Pass | Fail | Duration(ms) | Error |
|---|---|---|---|---|---|---|
| auth-flow | FAIL | 2 | 0 | 2 | 100 |  |
| workspace | FAIL | 1 | 0 | 1 | 25 |  |
| project-workbench | FAIL | 0 | 0 | 0 | 0 | project id is missing; workspace phase did not complete successfully |
| asset-hub | FAIL | 1 | 0 | 1 | 34 |  |
| profile | FAIL | 1 | 0 | 1 | 24 |  |
| admin-non-admin | FAIL | 1 | 0 | 1 | 33 |  |
| i18n-redirect | FAIL | 1 | 0 | 1 | 25 |  |
| not-found | FAIL | 1 | 0 | 1 | 36 |  |

## auth-flow

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| landing-route-render | FAIL | BROWSER | / |  | 73 | 0 | 0 | 0 | 1 | reports/frontend-e2e/01-auth-flow-landing-route-render-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/ Call log: [2m  - navigating to "http://127.0.0.1:9/", waiting until "domcontentloaded"[22m  |
| signup-form-render | FAIL | BROWSER | /auth/signup |  | 27 | 0 | 0 | 0 | 0 | reports/frontend-e2e/02-auth-flow-signup-form-render-fail.png | page.evaluate: SecurityError: Failed to read the 'localStorage' property from 'Window': Access is denied for this document.     at UtilityScript.evaluate (<anonymous>:292:16)     at UtilityScript.<anonymous> (<anonymous>:1:44) |

## workspace

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| workspace-list-render | FAIL | BROWSER | /workspace |  | 25 | 0 | 0 | 0 | 1 | reports/frontend-e2e/03-workspace-workspace-list-render-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/workspace Call log: [2m  - navigating to "http://127.0.0.1:9/workspace", waiting until "domcontentloaded"[22m  |

## project-workbench

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|

## asset-hub

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| asset-hub-load-without-crash | FAIL | BROWSER | /workspace/asset-hub |  | 34 | 0 | 0 | 0 | 1 | reports/frontend-e2e/04-asset-hub-asset-hub-load-without-crash-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/workspace/asset-hub Call log: [2m  - navigating to "http://127.0.0.1:9/workspace/asset-hub", waiting until "domcontentloaded"[22m  |

## profile

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| profile-route-load | FAIL | BROWSER | /profile |  | 24 | 0 | 0 | 0 | 1 | reports/frontend-e2e/05-profile-profile-route-load-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/profile Call log: [2m  - navigating to "http://127.0.0.1:9/profile", waiting until "domcontentloaded"[22m  |

## admin-non-admin

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| admin-ai-config-route-load | FAIL | BROWSER | /admin/ai-config |  | 33 | 0 | 0 | 0 | 1 | reports/frontend-e2e/06-admin-non-admin-admin-ai-config-route-load-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/admin/ai-config Call log: [2m  - navigating to "http://127.0.0.1:9/admin/ai-config", waiting until "domcontentloaded"[22m  |

## i18n-redirect

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| locale-zh-workspace-redirect | FAIL | BROWSER | /zh/workspace |  | 25 | 0 | 0 | 0 | 1 | reports/frontend-e2e/07-i18n-redirect-locale-zh-workspace-redirect-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/zh/workspace Call log: [2m  - navigating to "http://127.0.0.1:9/zh/workspace", waiting until "domcontentloaded"[22m  |

## not-found

| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| not-found-route | FAIL | BROWSER | /nonexistent-page |  | 36 | 0 | 0 | 0 | 1 | reports/frontend-e2e/08-not-found-not-found-route-fail.png | page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/nonexistent-page Call log: [2m  - navigating to "http://127.0.0.1:9/nonexistent-page", waiting until "domcontentloaded"[22m  |

## Cleanup

- no cleanup steps executed

## Artifacts

- Console log: reports/frontend-e2e/e2e-frontend-console.log
- Screenshot: reports/frontend-e2e/01-auth-flow-landing-route-render-fail.png
- Screenshot: reports/frontend-e2e/02-auth-flow-signup-form-render-fail.png
- Screenshot: reports/frontend-e2e/03-workspace-workspace-list-render-fail.png
- Screenshot: reports/frontend-e2e/04-asset-hub-asset-hub-load-without-crash-fail.png
- Screenshot: reports/frontend-e2e/05-profile-profile-route-load-fail.png
- Screenshot: reports/frontend-e2e/06-admin-non-admin-admin-ai-config-route-load-fail.png
- Screenshot: reports/frontend-e2e/07-i18n-redirect-locale-zh-workspace-redirect-fail.png
- Screenshot: reports/frontend-e2e/08-not-found-not-found-route-fail.png

## Failures

- auth-flow/landing-route-render: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/
Call log:
[2m  - navigating to "http://127.0.0.1:9/", waiting until "domcontentloaded"[22m

- auth-flow/signup-form-render: page.evaluate: SecurityError: Failed to read the 'localStorage' property from 'Window': Access is denied for this document.
    at UtilityScript.evaluate (<anonymous>:292:16)
    at UtilityScript.<anonymous> (<anonymous>:1:44)
- workspace/workspace-list-render: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/workspace
Call log:
[2m  - navigating to "http://127.0.0.1:9/workspace", waiting until "domcontentloaded"[22m

- project-workbench: project id is missing; workspace phase did not complete successfully
- asset-hub/asset-hub-load-without-crash: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/workspace/asset-hub
Call log:
[2m  - navigating to "http://127.0.0.1:9/workspace/asset-hub", waiting until "domcontentloaded"[22m

- profile/profile-route-load: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/profile
Call log:
[2m  - navigating to "http://127.0.0.1:9/profile", waiting until "domcontentloaded"[22m

- admin-non-admin/admin-ai-config-route-load: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/admin/ai-config
Call log:
[2m  - navigating to "http://127.0.0.1:9/admin/ai-config", waiting until "domcontentloaded"[22m

- i18n-redirect/locale-zh-workspace-redirect: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/zh/workspace
Call log:
[2m  - navigating to "http://127.0.0.1:9/zh/workspace", waiting until "domcontentloaded"[22m

- not-found/not-found-route: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:9/nonexistent-page
Call log:
[2m  - navigating to "http://127.0.0.1:9/nonexistent-page", waiting until "domcontentloaded"[22m


## Severe Diagnostics

- none

