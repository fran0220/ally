-- Graph runtime schema alignment with prisma/schema.prisma
-- Safe for repeated execution on MySQL 8.x.

CREATE TABLE IF NOT EXISTS `graph_artifacts` (
  `id` VARCHAR(191) NOT NULL,
  `runId` VARCHAR(191) NOT NULL,
  `stepKey` VARCHAR(191) NULL,
  `artifactType` VARCHAR(191) NOT NULL,
  `refId` VARCHAR(191) NOT NULL,
  `versionHash` VARCHAR(191) NULL,
  `payload` JSON NULL,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  INDEX `graph_artifacts_runId_idx`(`runId`),
  INDEX `graph_artifacts_runId_stepKey_idx`(`runId`, `stepKey`),
  INDEX `graph_artifacts_artifactType_refId_idx`(`artifactType`, `refId`),
  PRIMARY KEY (`id`),
  CONSTRAINT `graph_artifacts_runId_fkey` FOREIGN KEY (`runId`) REFERENCES `graph_runs`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE `graph_step_attempts`
  ADD COLUMN IF NOT EXISTS `provider` VARCHAR(191) NULL AFTER `status`,
  ADD COLUMN IF NOT EXISTS `modelKey` VARCHAR(191) NULL AFTER `provider`,
  ADD COLUMN IF NOT EXISTS `inputHash` VARCHAR(191) NULL AFTER `modelKey`,
  ADD COLUMN IF NOT EXISTS `input` JSON NULL AFTER `inputHash`;

SET @schema_name := DATABASE();

SET @has_graph_runs_taskid_unique := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_runs'
    AND index_name = 'graph_runs_taskId_key'
    AND non_unique = 0
);
SET @sql := IF(
  @has_graph_runs_taskid_unique > 0,
  'SELECT 1',
  'ALTER TABLE `graph_runs` ADD UNIQUE INDEX `graph_runs_taskId_key`(`taskId`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_events_runid_seq_unique := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_events'
    AND index_name = 'graph_events_runId_seq_idx'
    AND non_unique = 0
);
SET @has_graph_events_runid_seq_non_unique := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_events'
    AND index_name = 'graph_events_runId_seq_idx'
    AND non_unique = 1
);
SET @sql := IF(
  @has_graph_events_runid_seq_unique > 0,
  'SELECT 1',
  IF(
    @has_graph_events_runid_seq_non_unique > 0,
    'ALTER TABLE `graph_events` DROP INDEX `graph_events_runId_seq_idx`, ADD UNIQUE INDEX `graph_events_runId_seq_idx`(`runId`, `seq`)',
    'ALTER TABLE `graph_events` ADD UNIQUE INDEX `graph_events_runId_seq_idx`(`runId`, `seq`)'
  )
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_runs_project_status_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_runs'
    AND index_name = 'graph_runs_projectId_status_idx'
);
SET @sql := IF(
  @has_graph_runs_project_status_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_runs` ADD INDEX `graph_runs_projectId_status_idx`(`projectId`, `status`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_runs_user_created_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_runs'
    AND index_name = 'graph_runs_userId_createdAt_idx'
);
SET @sql := IF(
  @has_graph_runs_user_created_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_runs` ADD INDEX `graph_runs_userId_createdAt_idx`(`userId`, `createdAt`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_runs_target_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_runs'
    AND index_name = 'graph_runs_targetType_targetId_idx'
);
SET @sql := IF(
  @has_graph_runs_target_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_runs` ADD INDEX `graph_runs_targetType_targetId_idx`(`targetType`, `targetId`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_steps_run_status_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_steps'
    AND index_name = 'graph_steps_runId_status_idx'
);
SET @sql := IF(
  @has_graph_steps_run_status_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_steps` ADD INDEX `graph_steps_runId_status_idx`(`runId`, `status`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_step_attempts_run_step_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_step_attempts'
    AND index_name = 'graph_step_attempts_runId_stepKey_idx'
);
SET @sql := IF(
  @has_graph_step_attempts_run_step_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_step_attempts` ADD INDEX `graph_step_attempts_runId_stepKey_idx`(`runId`, `stepKey`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_step_attempts_run_created_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_step_attempts'
    AND index_name = 'graph_step_attempts_runId_createdAt_idx'
);
SET @sql := IF(
  @has_graph_step_attempts_run_created_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_step_attempts` ADD INDEX `graph_step_attempts_runId_createdAt_idx`(`runId`, `createdAt`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_events_run_id_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_events'
    AND index_name = 'graph_events_runId_id_idx'
);
SET @sql := IF(
  @has_graph_events_run_id_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_events` ADD INDEX `graph_events_runId_id_idx`(`runId`, `id`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_events_user_id_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_events'
    AND index_name = 'graph_events_userId_id_idx'
);
SET @sql := IF(
  @has_graph_events_user_id_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_events` ADD INDEX `graph_events_userId_id_idx`(`userId`, `id`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_graph_checkpoints_run_created_idx := (
  SELECT COUNT(*)
  FROM information_schema.statistics
  WHERE table_schema = @schema_name
    AND table_name = 'graph_checkpoints'
    AND index_name = 'graph_checkpoints_runId_createdAt_idx'
);
SET @sql := IF(
  @has_graph_checkpoints_run_created_idx > 0,
  'SELECT 1',
  'ALTER TABLE `graph_checkpoints` ADD INDEX `graph_checkpoints_runId_createdAt_idx`(`runId`, `createdAt`)'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;
