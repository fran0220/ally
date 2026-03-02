# crates/worker — Redis Stream 消费者

独立进程，从 Redis Streams 消费任务并执行。

## 架构

```
main.rs → spawn_queue_workers() → dispatcher → consumer → handler
                                                             ↓
                                                     heartbeat (独立协程)
```

## 队列

| 队列 | 默认并发 | 环境变量 |
|------|---------|----------|
| image | 20 | `QUEUE_CONCURRENCY_IMAGE` |
| text | 10 | `QUEUE_CONCURRENCY_TEXT` |
| video | 4 | `QUEUE_CONCURRENCY_VIDEO` |
| voice | 10 | `QUEUE_CONCURRENCY_VOICE` |

## 模块

| 文件 | 职责 |
|------|------|
| `main.rs` | Worker 启动，按队列类型派发协程 |
| `consumer.rs` | Redis Stream XREADGROUP 消费逻辑 |
| `dispatcher.rs` | 任务分发循环（消费 → 匹配 handler → 执行 → ACK） |
| `heartbeat.rs` | Worker 心跳上报 |
| `task_context.rs` | 任务上下文（DB/Redis pool + 任务元数据） |
| `runtime.rs` | DAG 运行时初始化 |
| `handlers/` | 各任务类型的处理函数（image_*, text_*, video_*, voice_*） |

## 新增 Handler

1. 在 `handlers/` 下新增函数
2. 在 `dispatcher.rs` 中注册 match 分支
3. 消费/持久化错误不应中断 worker 主循环
