-- Simplify billing to direct credit deductions without freeze/unfreeze.
-- Old tables (balance_freezes, usage_costs, balance_transactions) are kept intact
-- and should be treated as read-only historical data.

CREATE TABLE IF NOT EXISTS `model_pricing` (
  `id` VARCHAR(36) NOT NULL,
  `api_type` VARCHAR(32) NOT NULL,
  `model_id` VARCHAR(128) NOT NULL,
  `unit` VARCHAR(32) NOT NULL,
  `unit_price` DECIMAL(16, 6) NOT NULL,
  `description` VARCHAR(256) DEFAULT NULL,
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  UNIQUE KEY `uk_api_model_unit` (`api_type`, `model_id`, `unit`),
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `credit_records` (
  `id` VARCHAR(36) NOT NULL,
  `user_id` VARCHAR(36) NOT NULL,
  `type` ENUM('consume', 'recharge', 'refund', 'admin_adjust') NOT NULL,
  `amount` DECIMAL(16, 6) NOT NULL,
  `balance_after` DECIMAL(16, 6) NOT NULL,
  `api_type` VARCHAR(32) DEFAULT NULL,
  `model` VARCHAR(128) DEFAULT NULL,
  `action` VARCHAR(64) DEFAULT NULL,
  `quantity` DECIMAL(16, 6) DEFAULT NULL,
  `unit` VARCHAR(32) DEFAULT NULL,
  `unit_price` DECIMAL(16, 6) DEFAULT NULL,
  `project_id` VARCHAR(36) DEFAULT NULL,
  `episode_id` VARCHAR(36) DEFAULT NULL,
  `task_id` VARCHAR(36) DEFAULT NULL,
  `operator_id` VARCHAR(36) DEFAULT NULL,
  `external_order_id` VARCHAR(128) DEFAULT NULL,
  `idempotency_key` VARCHAR(128) DEFAULT NULL,
  `metadata` JSON DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  INDEX `idx_user_created` (`user_id`, `created_at`),
  INDEX `idx_project_created` (`project_id`, `created_at`),
  INDEX `idx_task` (`task_id`),
  UNIQUE KEY `uk_task_type` (`task_id`, `type`),
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET @schema_name := DATABASE();

-- Return all frozen funds before dropping the legacy frozenAmount column.
SET @has_frozen_amount := (
  SELECT COUNT(*)
  FROM information_schema.columns
  WHERE table_schema = @schema_name
    AND table_name = 'user_balances'
    AND column_name = 'frozenAmount'
);

SET @sql := IF(
  @has_frozen_amount > 0,
  'UPDATE `user_balances` SET `balance` = `balance` + `frozenAmount`, `frozenAmount` = 0 WHERE `frozenAmount` > 0',
  'SELECT 1'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql := IF(
  @has_frozen_amount > 0,
  'ALTER TABLE `user_balances` DROP COLUMN `frozenAmount`',
  'SELECT 1'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Seed model_pricing from standards/pricing/*.json.
-- LLM JSON amounts are per 1M tokens; convert to per-token unit prices.
-- Kling video tiers are linear by duration; normalize to per-second pricing.
INSERT INTO `model_pricing` (
  `id`,
  `api_type`,
  `model_id`,
  `unit`,
  `unit_price`,
  `description`
)
VALUES
  (UUID(), 'text', 'claude-sonnet-4-6', 'input_token', CAST(21.6 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible input token price'),
  (UUID(), 'text', 'claude-sonnet-4-6', 'output_token', CAST(108 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible output token price'),
  (UUID(), 'text', 'claude-opus-4-6', 'input_token', CAST(108 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible input token price'),
  (UUID(), 'text', 'claude-opus-4-6', 'output_token', CAST(540 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible output token price'),
  (UUID(), 'text', 'gemini-3.1-pro-preview', 'input_token', CAST(9 / 1000000 AS DECIMAL(16, 6)), 'gemini-compatible input token price'),
  (UUID(), 'text', 'gemini-3.1-pro-preview', 'output_token', CAST(72 / 1000000 AS DECIMAL(16, 6)), 'gemini-compatible output token price'),
  (UUID(), 'text', 'gemini-3-flash-preview', 'input_token', CAST(0.54 / 1000000 AS DECIMAL(16, 6)), 'gemini-compatible input token price'),
  (UUID(), 'text', 'gemini-3-flash-preview', 'output_token', CAST(2.16 / 1000000 AS DECIMAL(16, 6)), 'gemini-compatible output token price'),
  (UUID(), 'text', 'gpt-5.2', 'input_token', CAST(14.4 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible input token price'),
  (UUID(), 'text', 'gpt-5.2', 'output_token', CAST(43.2 / 1000000 AS DECIMAL(16, 6)), 'openai-compatible output token price'),

  (UUID(), 'image', 'banana', 'image', 0.964800, 'fal flat image price'),
  (UUID(), 'image', 'banana-2', 'image:1K', 0.576000, 'fal capability image price (1K)'),
  (UUID(), 'image', 'banana-2', 'image:2K', 0.864000, 'fal capability image price (2K)'),
  (UUID(), 'image', 'banana-2', 'image:4K', 1.152000, 'fal capability image price (4K)'),
  (UUID(), 'video', 'fal-wan25', 'video', 1.800000, 'fal flat video price'),
  (UUID(), 'video', 'fal-veo31', 'video', 2.880000, 'fal flat video price'),
  (UUID(), 'video', 'fal-sora2', 'video', 3.600000, 'fal flat video price'),
  (UUID(), 'video', 'fal-ai/kling-video/v2.5-turbo/pro/image-to-video', 'second', 0.070000, 'fal kling v2.5 normalized per-second price'),
  (UUID(), 'video', 'fal-ai/kling-video/v3/standard/image-to-video', 'second', 0.168000, 'fal kling v3 standard normalized per-second price'),
  (UUID(), 'video', 'fal-ai/kling-video/v3/pro/image-to-video', 'second', 0.224000, 'fal kling v3 pro normalized per-second price'),
  (UUID(), 'voice', 'fal-ai/index-tts-2/text-to-speech', 'second', 0.014400, 'fal voice synthesis per-second price'),
  (UUID(), 'voice-design', 'qwen', 'call', 0.200000, 'qwen voice design per-call price'),
  (UUID(), 'lip-sync', 'fal-ai/kling-video/lipsync/audio-to-video', 'call', 0.500000, 'fal lip-sync per-call price')
ON DUPLICATE KEY UPDATE
  `unit_price` = VALUES(`unit_price`),
  `description` = VALUES(`description`),
  `updated_at` = CURRENT_TIMESTAMP(3);
