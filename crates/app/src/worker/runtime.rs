use once_cell::sync::OnceCell;
use sqlx::MySqlPool;
use waoowaoo_core::errors::AppError;

static MYSQL_POOL: OnceCell<MySqlPool> = OnceCell::new();

pub fn init(mysql: MySqlPool) -> Result<(), AppError> {
    MYSQL_POOL
        .set(mysql)
        .map_err(|_| AppError::internal("worker runtime already initialized"))
}

pub fn mysql() -> Result<&'static MySqlPool, AppError> {
    MYSQL_POOL
        .get()
        .ok_or_else(|| AppError::internal("worker runtime mysql is not initialized"))
}
