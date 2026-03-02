# crates/ — Rust Workspace

4 个 crate 组成的 Cargo workspace，共享 `workspace.dependencies`。

## Crate 依赖关系

```
core ← server (HTTP API)
core ← worker (任务处理)
core ← watchdog (超时扫描)
```

`core` 是所有其他 crate 的依赖，修改 `core` 的公共 API 时需检查下游 crate 编译。

## 规范

- **Edition**: 2024
- **新增依赖**: 必须添加到 `[workspace.dependencies]`，crate 内用 `dep.workspace = true`
- **错误处理**: core 用 `thiserror` 定义错误类型；server/worker 用 `anyhow::Result`
- **异步**: 统一 Tokio runtime，不混用 async-std
- **日志**: 使用 `tracing::{info, warn, error, debug}` 宏，不使用 `println!`
- **测试**: `#[cfg(test)] mod tests` 放在模块文件底部
- **序列化**: 统一 `serde` + `serde_json`，MySQL 日期用 `chrono`

## 验证

```bash
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```
