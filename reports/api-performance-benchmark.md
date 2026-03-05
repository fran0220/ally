# API Performance Benchmark Report

- Generated at: 2026-03-04T08:31:13.537Z
- Duration per case: 5000ms
- Concurrency: 2
- Request timeout: 10000ms
- Warmup requests per side: 3
- Total: 7 | Executed: 6 | Skipped: 1 | Pass: 1 | Fail: 5

| Case | Result | Legacy RPS | Rust RPS | RPS Δ% | Legacy P95(ms) | Rust P95(ms) | P95 Δ% | Legacy Success% | Rust Success% |
|---|---|---|---|---|---|---|---|---|---|
| system-healthz | FAIL | 33.6 | 7.8 | -76.79 | 72.46 | 255.74 | 252.94 | 0 | 100 |
| system-boot-id | PASS | 8 | 8 | 0 | 257.22 | 253.03 | -1.63 | 100 | 100 |
| projects-list-auth | FAIL | 8.4 | 7.8 | -7.14 | 274.67 | 256.25 | -6.71 | 0 | 100 |
| tasks-list-auth | FAIL | 8 | 8 | 0 | 260.86 | 254.47 | -2.45 | 0 | 100 |
| runs-list-auth | FAIL | 7.2 | 7.8 | 8.33 | 841.47 | 260.58 | -69.03 | 0 | 100 |
| asset-hub-picker-auth | FAIL | 8 | 8 | 0 | 265.13 | 254.78 | -3.9 | 0 | 100 |

## Skipped Cases

- novel-root-auth (GET /api/novel-promotion/{{WW_PROJECT_ID}}) reason: missing env: WW_PROJECT_ID

## Failures

- system-healthz (GET /healthz)
  - legacy side has zero successful responses
- projects-list-auth (GET /api/projects?page=1&pageSize=5)
  - legacy side has zero successful responses
- tasks-list-auth (GET /api/tasks?limit=5)
  - legacy side has zero successful responses
- runs-list-auth (GET /api/runs?limit=5)
  - legacy side has zero successful responses
- asset-hub-picker-auth (GET /api/asset-hub/picker?type=character)
  - legacy side has zero successful responses

