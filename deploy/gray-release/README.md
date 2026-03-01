# Gray Release Config Templates

This directory contains canary traffic-splitting templates for the migration period:

- `nginx-percent-split.conf`: strict percentage split using `split_clients`
- `Caddyfile.percent-split`: weighted split for Caddy using repeated upstream entries

## Upstream Mapping

- `127.0.0.1:3000`: legacy Next.js full-stack
- `127.0.0.1:3001`: Rust Axum API
- `127.0.0.1:5173`: Vite frontend (optional for production)

## Nginx Notes

1. Update the `split_clients` ratio (`10%` by default) for canary rollout.
2. `/api/*` and `/m/*` are split between legacy and Rust.
3. The `X-Waoowaoo-Canary: rust|next` header can force routing for debugging.

## Caddy Notes

1. Caddy core does not provide native percentage split directives.
2. This template uses repeated upstream entries (9x legacy + 1x rust ~= 10%).
3. Use `X-Waoowaoo-Canary: rust|next` to force routing during validation.
