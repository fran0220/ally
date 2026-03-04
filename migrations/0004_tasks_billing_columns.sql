-- Add billing columns to tasks table
-- MySQL 8.0 compatible (no IF NOT EXISTS for ADD COLUMN)
-- Check column existence before running if re-executing

SET @has_billing_info := (
  SELECT COUNT(*)
  FROM information_schema.columns
  WHERE table_schema = DATABASE()
    AND table_name = 'tasks'
    AND column_name = 'billingInfo'
);
SET @sql := IF(
  @has_billing_info > 0,
  'SELECT 1',
  'ALTER TABLE `tasks` ADD COLUMN `billingInfo` JSON NULL AFTER `errorMessage`'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @has_billed_at := (
  SELECT COUNT(*)
  FROM information_schema.columns
  WHERE table_schema = DATABASE()
    AND table_name = 'tasks'
    AND column_name = 'billedAt'
);
SET @sql := IF(
  @has_billed_at > 0,
  'SELECT 1',
  'ALTER TABLE `tasks` ADD COLUMN `billedAt` DATETIME(3) NULL AFTER `billingInfo`'
);
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;
