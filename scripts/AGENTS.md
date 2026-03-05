# scripts/ — 测试与验证脚本

所有脚本为 Node.js ESM（`.mjs`），用于 Phase 8 联调验收。

## 脚本清单

| 脚本 | 用途 | 运行方式 |
|------|------|----------|
| `api-contract-regression.mjs` | 静态 API 契约扫描（Next.js vs Rust 路由源码对比） | `node scripts/api-contract-regression.mjs` |
| `api-runtime-compare.mjs` | 运行时 API 行为对比（legacy vs rust 双端） | `node scripts/api-runtime-compare.mjs --legacy-base ... --rust-base ... --token ... --cases ...` |
| `api-runtime-cases.sample.json` | 运行时对比用例样本（59 条） | 配合 api-runtime-compare.mjs |
| `api-performance-benchmark.mjs` | 性能基准测试（延迟/吞吐对比） | `node scripts/api-performance-benchmark.mjs --legacy-base ... --rust-base ...` |
| `api-performance-cases.sample.json` | 性能测试用例样本 | 配合 benchmark 脚本 |
| `worker-runtime-smoke.mjs` | Worker 任务烟测（image/text/video/voice） | `node scripts/worker-runtime-smoke.mjs --base ...` |
| `worker-runtime-cases.sample.json` | Worker 烟测用例 | 配合 worker smoke 脚本 |
| `sse-reconnect-smoke.mjs` | SSE 断线重连测试 | `node scripts/sse-reconnect-smoke.mjs --base ... --project-id ...` |
| `e2e-run-all.mjs` | 一键串行 E2E（auth-flow → project-lifecycle → asset-hub-crud → novel-promotion → user-config → admin-billing）并汇总报告 | `node scripts/e2e-run-all.mjs --base ... [--clean]` |

## 注意

- 用例文件支持 `{{ENV_VAR}}` 占位符
- 缺失的用例自动跳过（不报错）
- 运行时测试需要 legacy 和 rust 服务同时在线
