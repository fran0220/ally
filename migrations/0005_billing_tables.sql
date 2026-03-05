-- Create billing tables if they don't exist
-- These were defined in 0001_initial.sql but may be missing on servers
-- that were set up before billing was implemented.

CREATE TABLE IF NOT EXISTS `usage_costs` (
    `id` VARCHAR(191) NOT NULL,
    `projectId` VARCHAR(191) NOT NULL,
    `userId` VARCHAR(191) NOT NULL,
    `apiType` VARCHAR(191) NOT NULL,
    `model` VARCHAR(191) NOT NULL,
    `action` VARCHAR(191) NOT NULL,
    `quantity` INTEGER NOT NULL,
    `unit` VARCHAR(191) NOT NULL,
    `cost` DECIMAL(18, 6) NOT NULL,
    `metadata` TEXT NULL,
    `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX `usage_costs_apiType_idx`(`apiType`),
    INDEX `usage_costs_createdAt_idx`(`createdAt`),
    INDEX `usage_costs_projectId_idx`(`projectId`),
    INDEX `usage_costs_userId_idx`(`userId`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `user_balances` (
    `id` VARCHAR(191) NOT NULL,
    `userId` VARCHAR(191) NOT NULL,
    `balance` DECIMAL(18, 6) NOT NULL DEFAULT 0,
    `frozenAmount` DECIMAL(18, 6) NOT NULL DEFAULT 0,
    `totalSpent` DECIMAL(18, 6) NOT NULL DEFAULT 0,
    `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    `updatedAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    UNIQUE INDEX `user_balances_userId_key`(`userId`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `balance_freezes` (
    `id` VARCHAR(191) NOT NULL,
    `userId` VARCHAR(191) NOT NULL,
    `amount` DECIMAL(18, 6) NOT NULL,
    `status` VARCHAR(191) NOT NULL DEFAULT 'pending',
    `source` VARCHAR(64) NULL,
    `taskId` VARCHAR(191) NULL,
    `requestId` VARCHAR(191) NULL,
    `idempotencyKey` VARCHAR(191) NULL,
    `metadata` TEXT NULL,
    `expiresAt` DATETIME(3) NULL,
    `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    `updatedAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    UNIQUE INDEX `balance_freezes_idempotencyKey_key`(`idempotencyKey`),
    INDEX `balance_freezes_userId_idx`(`userId`),
    INDEX `balance_freezes_status_idx`(`status`),
    INDEX `balance_freezes_taskId_idx`(`taskId`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `balance_transactions` (
    `id` VARCHAR(191) NOT NULL,
    `userId` VARCHAR(191) NOT NULL,
    `type` VARCHAR(191) NOT NULL,
    `amount` DECIMAL(18, 6) NOT NULL,
    `balanceAfter` DECIMAL(18, 6) NOT NULL,
    `description` TEXT NULL,
    `relatedId` VARCHAR(191) NULL,
    `freezeId` VARCHAR(191) NULL,
    `operatorId` VARCHAR(64) NULL,
    `externalOrderId` VARCHAR(128) NULL,
    `idempotencyKey` VARCHAR(128) NULL,
    `projectId` VARCHAR(128) NULL,
    `episodeId` VARCHAR(128) NULL,
    `taskType` VARCHAR(64) NULL,
    `billingMeta` TEXT NULL,
    `createdAt` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX `balance_transactions_userId_idx`(`userId`),
    INDEX `balance_transactions_type_idx`(`type`),
    INDEX `balance_transactions_createdAt_idx`(`createdAt`),
    INDEX `balance_transactions_freezeId_idx`(`freezeId`),
    INDEX `balance_transactions_externalOrderId_idx`(`externalOrderId`),
    INDEX `balance_transactions_projectId_idx`(`projectId`),
    UNIQUE INDEX `balance_transactions_userId_type_idempotencyKey_key`(`userId`, `type`, `idempotencyKey`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
