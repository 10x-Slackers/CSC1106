pub const PER_PAGE: u64 = 10;

#[derive(Clone, Copy, serde::Serialize)]
pub struct Pagination {
    pub current: u32,
    pub total_pages: u32,
}

pub fn parse_page(raw: Option<u32>) -> u32 {
    raw.unwrap_or(1).max(1)
}

pub fn clamp_page(page: u32, total_pages: u32) -> u32 {
    page.min(total_pages.max(1))
}

/// Serialize `filter` to a query string, then strip any `page=...` pair.
/// Returns `""` if the result is empty or only contained `page`.
/// The returned string has no leading `?` and no trailing `&`.
pub fn base_query_string<T: serde::Serialize>(filter: &T) -> String {
    let qs = match serde_qs::to_string(filter) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };

    let stripped: Vec<&str> = qs
        .split('&')
        .filter(|pair| {
            let key = pair.split('=').next().unwrap_or("");
            key != "page"
        })
        .collect();

    if stripped.is_empty() {
        String::new()
    } else {
        stripped.join("&")
    }
}
