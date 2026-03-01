# API Performance Benchmark Report

- Generated at: 2026-03-01T13:43:45.214Z
- Duration per case: 5000ms
- Concurrency: 2
- Request timeout: 5000ms
- Warmup requests per side: 1
- Total: 7 | Executed: 7 | Skipped: 0 | Pass: 5 | Fail: 2

| Case | Result | Legacy RPS | Rust RPS | RPS Δ% | Legacy P95(ms) | Rust P95(ms) | P95 Δ% | Legacy Success% | Rust Success% |
|---|---|---|---|---|---|---|---|---|---|
| system-healthz | FAIL | 26.2 | 13087.6 | 49852.67 | 121.3 | 0.32 | -99.74 | 0 | 100 |
| system-boot-id | PASS | 7.6 | 11709.2 | 153968.42 | 319.05 | 0.37 | -99.88 | 100 | 100 |
| projects-list-auth | PASS | 1.4 | 1.6 | 14.29 | 2711.29 | 1719.13 | -36.59 | 100 | 100 |
| tasks-list-auth | PASS | 3.4 | 2.6 | -23.53 | 946.52 | 1035.33 | 9.38 | 100 | 100 |
| runs-list-auth | FAIL | 8 | 2.6 | -67.5 | 264.95 | 1032.64 | 289.75 | 0 | 100 |
| asset-hub-picker-auth | PASS | 3.4 | 3 | -11.76 | 1268.22 | 1012.95 | -20.13 | 100 | 100 |
| novel-root-auth | PASS | 2 | 0.8 | -60 | 1274.12 | 4439.67 | 248.45 | 100 | 100 |

## Skipped Cases

- none

## Failures

- system-healthz (GET /healthz)
  - legacy side has zero successful responses
- runs-list-auth (GET /api/runs?limit=5)
  - legacy side has zero successful responses

