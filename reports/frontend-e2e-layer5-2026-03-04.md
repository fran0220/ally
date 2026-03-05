# Layer 5 Frontend E2E Validation (2026-03-04)

## Scope

- Target: `http://185.200.65.233` (production SPA via Nginx)
- Method: `mcp__next_devtools__browser_eval` browser automation
- Account used: `e2e-test-1772612785@test.com` / `Test123456`
- Test project created: `e2e-project-1772612785` (`id: 5e53d4fc-d139-4e07-a129-773066480653`)

## Flow Results

| # | Flow | Result | Details |
|---|------|--------|---------|
| 1 | Auth (`/auth/signup` + `/auth/login`) | **FAIL (partial)** | Signup succeeded and auto-redirected to `/workspace`. However `/auth/login` is **404 Page not found** in current SPA routes. Login works on `/auth/signin` with same credentials and redirects to `/workspace`. |
| 2 | Workspace (`/workspace`) | **PASS** | Project list loaded. Created project `e2e-project-1772612785`; project name appears in workspace list and `GET /api/projects` includes the project record. |
| 3 | Project workbench (`/workspace/:id`) | **FAIL** | Route crashes to React Router error boundary. Browser console shows `TypeError: Failed to construct 'URL': Invalid base URL` from `use-task-sse-*.js`; 6 stage tabs cannot be verified because page fails before render. |
| 4 | Asset Hub (`/workspace/asset-hub`) | **FAIL** | Same runtime crash (`Invalid base URL` from `use-task-sse-*.js`) causes error boundary; character/location/voice sections and CRUD cannot be exercised. |
| 5 | Profile (`/profile`) | **PASS** | API Config page loads successfully with provider/model configuration UI visible. |
| 6 | Admin (`/admin/ai-config`) | **PASS (unauthorized state)** | Page does not load admin config for this test account. Console shows `403 Forbidden` on `GET /api/admin/ai-config`, indicating non-admin access is blocked. UI shows `No config loaded`. |

## Key Findings

1. **Critical frontend runtime issue on workbench and asset hub**: `Failed to construct 'URL': Invalid base URL` breaks both routes.
2. **Route mismatch with requested login path**: `/auth/login` is not registered in SPA (404), while `/auth/signin` is the active login route.
3. **Core auth + workspace baseline works**: signup, authenticated workspace entry, and project creation are functional.

## Evidence Artifacts

- Signup page: `reports/layer5-e2e-2026-03-04/01-auth-signup-page.png`
- Signup redirect to workspace: `reports/layer5-e2e-2026-03-04/02-auth-signup-redirect-workspace.png`
- Workspace project created: `reports/layer5-e2e-2026-03-04/03-workspace-project-created.png`
- Project workbench crash: `reports/layer5-e2e-2026-03-04/04-project-workbench-invalid-base-url.png`
- Asset hub crash: `reports/layer5-e2e-2026-03-04/05-asset-hub-invalid-base-url.png`
- Profile API config: `reports/layer5-e2e-2026-03-04/06-profile-api-config-loaded.png`
- Admin unauthorized state: `reports/layer5-e2e-2026-03-04/07-admin-ai-config-forbidden.png`
- `/auth/login` route 404: `reports/layer5-e2e-2026-03-04/08-auth-login-route-not-found.png`
- `/auth/signin` login success: `reports/layer5-e2e-2026-03-04/09-auth-signin-login-success.png`
- Workbench console error log: `reports/layer5-e2e-2026-03-04/console-project-workbench-error.log`
- Asset hub console error log: `reports/layer5-e2e-2026-03-04/console-asset-hub-error.log`
- Admin 403 console log: `reports/layer5-e2e-2026-03-04/console-admin-403.log`

## Notes

- This run created persistent production test data (user + project).
- If needed, clean up `e2e-test-1772612785@test.com` and project `e2e-project-1772612785` after analysis.
