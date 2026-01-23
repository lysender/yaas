use core::fmt;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::pagination::PaginatedMetaBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginatedMeta {
    pub page: i32,
    pub per_page: i32,
    pub total_records: i64,
    pub total_pages: i64,
}

impl From<PaginatedMetaBuf> for PaginatedMeta {
    fn from(meta: PaginatedMetaBuf) -> Self {
        PaginatedMeta {
            page: meta.page,
            per_page: meta.per_page,
            total_records: meta.total_records,
            total_pages: meta.total_pages,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub meta: PaginatedMeta,
    pub data: Vec<T>,
}

impl PaginatedMeta {
    pub fn new(page: i32, per_page: i32, total_records: i64) -> Self {
        let total_pages = (total_records as f64 / per_page as f64).ceil() as i64;
        let actual_page = if page <= total_pages as i32 { page } else { 1 };
        Self {
            page: actual_page,
            per_page,
            total_records,
            total_pages,
        }
    }
}

impl<T> Paginated<T> {
    pub fn new(records: Vec<T>, page: i32, per_page: i32, total_records: i64) -> Self {
        Self {
            meta: PaginatedMeta::new(page, per_page, total_records),
            data: records,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: i32,
    pub per_page: i32,
    pub offset: i64,
    pub total_records: i64,
    pub total_pages: i64,
}

impl PaginationParams {
    pub fn new(
        total_records: i64,
        page_param: Option<i32>,
        per_page_param: Option<i32>,
        max_per_page_param: Option<i32>,
    ) -> Self {
        let mut page: i32 = 1;
        let max_per_page: i32 = max_per_page_param.unwrap_or(50);
        let mut per_page: i32 = max_per_page;
        let mut offset: i64 = 0;

        if let Some(per_page_param) = per_page_param
            && per_page_param > 0 && per_page_param <= max_per_page {
                per_page = per_page_param;
            }

        let total_pages: i64 = (total_records as f64 / per_page as f64).ceil() as i64;

        if let Some(p) = page_param {
            let p64 = p as i64;
            if p64 > 0 && p64 <= total_pages {
                page = p;
                offset = (p64 - 1) * per_page as i64;
            }
        }
        Self {
            page,
            per_page,
            offset,
            total_records,
            total_pages,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListingParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,
}

impl Default for ListingParamsDto {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListingParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(f, "page={}&per_page={}", page, per_page,)
    }
}
