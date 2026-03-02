# deploy/ — 部署配置

## 目标服务器

| 别名 | IP | 用途 |
|------|-----|------|
| jpdata | 185.200.65.233 | 生产部署服务器（SSH root） |

## jpdata 服务架构

```
/opt/ally/
├── bin/                 # Rust 编译产物
│   ├── waoowaoo-server  # Axum HTTP API (:3001)
│   ├── waoowaoo-worker  # Redis Stream 消费者
│   └── waoowaoo-watchdog # 超时扫描
├── src/                 # 项目源码（rsync 同步）
├── frontend/dist/       # Vite SPA 产物
├── uploads/             # 用户上传文件
├── logs/                # 日志目录
└── .env                 # 环境变量

Nginx (:80)
  ├── /api/ → Rust Axum (:3001)
  ├── /api/events/ → SSE 长连接
  ├── /m/ → 媒体代理
  ├── /uploads/ → 静态文件
  ├── /healthz → 健康检查
  └── / → SPA (frontend/dist/)

Docker:
  ├── ally-mysql (MySQL 8.0, :3306)
  └── ally-redis (Redis 7, :6379)

Systemd:
  ├── ally-server.service
  ├── ally-worker.service
  └── ally-watchdog.service
```

## CI/CD 流程

### GitHub Actions (push to main)

```
check job:
  cargo fmt --check → clippy → test (MySQL + Redis services)
  tsc --noEmit → npm run build

deploy job (main only):
  npm run build → rsync src + dist → cargo build --release on server
  → cp binaries to /opt/ally/bin/ → systemctl restart → health check
```

### 手动部署

```bash
bash deploy/deploy.sh
```

## 文件清单

| 文件 | 用途 |
|------|------|
| `docker-compose.infra.yml` | MySQL + Redis 基础设施（使用外部 volume 保留数据） |
| `nginx-ally.conf` | Nginx 反代配置 |
| `ally-server.service` | systemd unit — Axum HTTP 服务 |
| `ally-worker.service` | systemd unit — Redis Stream Worker |
| `ally-watchdog.service` | systemd unit — 超时扫描 |
| `deploy.sh` | 本地一键部署脚本 |
| `setup-server.sh` | 首次服务器初始化 |
| `gray-release/` | 灰度发布模板（Nginx/Caddy 百分比切流） |

## 运维命令

```bash
# 查看服务状态
ssh jpdata 'systemctl status ally-server ally-worker ally-watchdog'

# 查看日志
ssh jpdata 'journalctl -u ally-server -f'
ssh jpdata 'journalctl -u ally-worker -f'

# 重启
ssh jpdata 'systemctl restart ally-server ally-worker ally-watchdog'

# Docker 基础设施
ssh jpdata 'cd /opt/ally/src && docker-compose -f deploy/docker-compose.infra.yml up -d'
```

## jpdata 上其他项目

| 项目 | 位置 | 状态 |
|------|------|------|
| jimeng-gateway | `/opt/jimeng-gateway/` (systemd) | ✅ 运行中 |
| jimeng-free-api | Docker `:8000` | ✅ 运行中 |
| asd-app (agent-behavior.com) | `/root/asd-app/` (Nginx) | ⚠️ PM2 未启动 |
| archon | `/root/archon/` | 未运行 |
| openai-proxy | Nginx `:8318` | ✅ 运行中 |
| frps | Docker | ✅ 运行中 |
