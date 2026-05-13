use crate::{api::PageRequest, cli::SortDirection};

pub fn page_request(
    limit: u16,
    start: u32,
    all: bool,
    sort: Option<String>,
    direction: Option<SortDirection>,
) -> PageRequest {
    PageRequest {
        limit,
        start,
        all,
        sort,
        direction: direction.map(|direction| direction.as_api_str().to_owned()),
    }
}
