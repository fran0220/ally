use once_cell::sync::Lazy;
use uuid::Uuid;

pub static SERVER_BOOT_ID: Lazy<String> = Lazy::new(|| format!("boot-{}", Uuid::new_v4()));
