# migrations/ — SQL 迁移文件

SQLx MySQL 迁移，手动管理（非自动迁移工具）。

## 文件

| 文件 | 内容 |
|------|------|
| `0001_initial.sql` | 初始表结构（users, projects, tasks, runs 等） |
| `0002_run_runtime.sql` | DAG 运行时表（graph_* 系列） |

## 规范

- 文件名：`NNNN_description.sql`，序号递增
- 必须是幂等的（`CREATE TABLE IF NOT EXISTS`，`ALTER TABLE ... ADD COLUMN IF NOT EXISTS`）
- 在 jpdata 上通过 Prisma 或直接 MySQL CLI 执行：
  ```bash
  mysql -u root -p waoowaoo < migrations/NNNN_xxx.sql
  ```
