# waoowaoo-rust

Rust migration workspace for `waoowaoo`.

## crates
- `crates/core`: shared business foundation (`config`, `errors`, `auth`, `db`, task/runtime services).
- `crates/server`: Axum HTTP server.
- `crates/worker`: task worker process with per-task handlers.
- `crates/watchdog`: timeout scanner process.

## quick start
```bash
cd waoowaoo-rust
cp .env.example .env
cargo run -p waoowaoo-server
```

## validation
```bash
cargo fmt
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

## phase 8 api regression
```bash
# static contract scan (Next.js route source vs Rust route source)
node scripts/api-contract-regression.mjs

# runtime contract compare dry-run (validate case file only)
node scripts/api-runtime-compare.mjs --dry-run

# runtime contract compare (legacy Next.js vs Rust)
node scripts/api-runtime-compare.mjs \
  --legacy-base http://127.0.0.1:3000 \
  --rust-base http://127.0.0.1:8080 \
  --token "<jwt>" \
  --cases scripts/api-runtime-cases.sample.json
```
