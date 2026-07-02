use serde::{Deserialize, Serialize};

/// 通用分页查询参数
///
/// 所有列表接口统一使用 `page` + `page_size` 分页。
/// - `page` 从 1 开始（默认 1）
/// - `page_size` 每页条数（默认 20，最大 100）
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

impl Pagination {
    /// 返回 (offset, limit) 供 SQL 使用
    pub fn offset_limit(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * page_size;
        (offset, page_size)
    }

    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 100)
    }
}

/// 分页响应包装
#[derive(Debug, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PagedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let p = Pagination { page: None, page_size: None };
        assert_eq!(p.page(), 1);
        assert_eq!(p.page_size(), 20);
        let (offset, limit) = p.offset_limit();
        assert_eq!(offset, 0);
        assert_eq!(limit, 20);
    }

    #[test]
    fn test_pagination_custom() {
        let p = Pagination { page: Some(3), page_size: Some(10) };
        assert_eq!(p.page(), 3);
        assert_eq!(p.page_size(), 10);
        let (offset, limit) = p.offset_limit();
        assert_eq!(offset, 20);
        assert_eq!(limit, 10);
    }

    #[test]
    fn test_pagination_clamp() {
        let p = Pagination { page: Some(0), page_size: None };
        assert_eq!(p.page(), 1);

        let p = Pagination { page: None, page_size: Some(500) };
        assert_eq!(p.page_size(), 100);

        let p = Pagination { page: None, page_size: Some(-5) };
        assert_eq!(p.page_size(), 1);
    }

    #[test]
    fn test_paged_response_total_pages() {
        let data: Vec<i32> = vec![1, 2, 3];
        let resp = PagedResponse::new(data, 25, 1, 10);
        assert_eq!(resp.total_pages, 3);
        assert_eq!(resp.total, 25);
        assert_eq!(resp.page, 1);
        assert_eq!(resp.page_size, 10);
    }

    #[test]
    fn test_paged_response_exact_division() {
        let data: Vec<i32> = vec![1, 2, 3, 4, 5];
        let resp = PagedResponse::new(data, 20, 1, 10);
        assert_eq!(resp.total_pages, 2);
    }

    #[test]
    fn test_paged_response_empty() {
        let data: Vec<i32> = vec![];
        let resp = PagedResponse::new(data, 0, 1, 20);
        assert_eq!(resp.total_pages, 0);
        assert!(resp.data.is_empty());
    }
}
