-- Run runtime tables (graph_*), aligned with existing TypeScript runtime contract.

CREATE TABLE IF NOT EXISTS `graph_runs` (
  `id` VARCHAR(191) NOT NULL,
  `userId` VARCHAR(191) NOT NULL,
  `projectId` VARCHAR(191) NOT NULL,
  `episodeId` VARCHAR(191) NULL,
  `workflowType` VARCHAR(191) NOT NULL,
  `taskType` VARCHAR(191) NULL,
  `taskId` VARCHAR(191) NULL,
  `targetType` VARCHAR(191) NOT NULL,
  `targetId` VARCHAR(191) NOT NULL,
  `status` VARCHAR(191) NOT NULL DEFAULT 'queued',
  `input` JSON NULL,
  `output` JSON NULL,
  `errorCode` VARCHAR(191) NULL,
  `errorMessage` TEXT NULL,
  `cancelRequestedAt` DATETIME(3) NULL,
  `queuedAt` DATETIME(3) NOT NULL,
  `startedAt` DATETIME(3) NULL,
  `finishedAt` DATETIME(3) NULL,
  `lastSeq` INTEGER NOT NULL DEFAULT 0,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updatedAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  INDEX `graph_runs_userId_idx`(`userId`),
  INDEX `graph_runs_projectId_idx`(`projectId`),
  INDEX `graph_runs_status_idx`(`status`),
  INDEX `graph_runs_createdAt_idx`(`createdAt`),
  PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `graph_steps` (
  `id` VARCHAR(191) NOT NULL,
  `runId` VARCHAR(191) NOT NULL,
  `stepKey` VARCHAR(191) NOT NULL,
  `stepTitle` VARCHAR(191) NOT NULL,
  `status` VARCHAR(191) NOT NULL DEFAULT 'pending',
  `currentAttempt` INTEGER NOT NULL DEFAULT 1,
  `stepIndex` INTEGER NOT NULL DEFAULT 1,
  `stepTotal` INTEGER NOT NULL DEFAULT 1,
  `startedAt` DATETIME(3) NULL,
  `finishedAt` DATETIME(3) NULL,
  `lastErrorCode` VARCHAR(191) NULL,
  `lastErrorMessage` TEXT NULL,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updatedAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  UNIQUE INDEX `graph_steps_runId_stepKey_key`(`runId`, `stepKey`),
  INDEX `graph_steps_runId_stepIndex_idx`(`runId`, `stepIndex`),
  PRIMARY KEY (`id`),
  CONSTRAINT `graph_steps_runId_fkey` FOREIGN KEY (`runId`) REFERENCES `graph_runs`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `graph_step_attempts` (
  `id` VARCHAR(191) NOT NULL,
  `runId` VARCHAR(191) NOT NULL,
  `stepKey` VARCHAR(191) NOT NULL,
  `attempt` INTEGER NOT NULL,
  `status` VARCHAR(191) NOT NULL,
  `outputText` LONGTEXT NULL,
  `outputReasoning` LONGTEXT NULL,
  `errorCode` VARCHAR(191) NULL,
  `errorMessage` TEXT NULL,
  `startedAt` DATETIME(3) NULL,
  `finishedAt` DATETIME(3) NULL,
  `usageJson` JSON NULL,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updatedAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  UNIQUE INDEX `graph_step_attempts_runId_stepKey_attempt_key`(`runId`, `stepKey`, `attempt`),
  INDEX `graph_step_attempts_runId_idx`(`runId`),
  PRIMARY KEY (`id`),
  CONSTRAINT `graph_step_attempts_runId_fkey` FOREIGN KEY (`runId`) REFERENCES `graph_runs`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `graph_events` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `runId` VARCHAR(191) NOT NULL,
  `projectId` VARCHAR(191) NOT NULL,
  `userId` VARCHAR(191) NOT NULL,
  `seq` INTEGER NOT NULL,
  `eventType` VARCHAR(191) NOT NULL,
  `stepKey` VARCHAR(191) NULL,
  `attempt` INTEGER NULL,
  `lane` VARCHAR(32) NULL,
  `payload` JSON NULL,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  INDEX `graph_events_runId_seq_idx`(`runId`, `seq`),
  INDEX `graph_events_projectId_id_idx`(`projectId`, `id`),
  PRIMARY KEY (`id`),
  CONSTRAINT `graph_events_runId_fkey` FOREIGN KEY (`runId`) REFERENCES `graph_runs`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `graph_checkpoints` (
  `id` VARCHAR(191) NOT NULL,
  `runId` VARCHAR(191) NOT NULL,
  `nodeKey` VARCHAR(191) NOT NULL,
  `version` INTEGER NOT NULL,
  `stateJson` JSON NOT NULL,
  `stateBytes` INTEGER NOT NULL,
  `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  UNIQUE INDEX `graph_checkpoints_runId_nodeKey_version_key`(`runId`, `nodeKey`, `version`),
  INDEX `graph_checkpoints_runId_nodeKey_idx`(`runId`, `nodeKey`),
  PRIMARY KEY (`id`),
  CONSTRAINT `graph_checkpoints_runId_fkey` FOREIGN KEY (`runId`) REFERENCES `graph_runs`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
