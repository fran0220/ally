use axum::extract::Query;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size", rename = "pageSize")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

impl Pagination {
    pub fn clamp(self) -> Self {
        Self {
            page: self.page.max(1),
            page_size: self.page_size.clamp(1, 200),
        }
    }
}

pub type PaginationQuery = Query<Pagination>;
