use super::*;

#[test]
fn builds_items_endpoints() {
    assert_eq!(
        ItemsRequest {
            collection: None,
            page: page(25),
            tag: None,
            top: false,
            trash: false,
        }
        .endpoint()
        .expect("endpoint"),
        "items"
    );
    assert_eq!(
        ItemsRequest {
            collection: None,
            page: page(25),
            tag: None,
            top: true,
            trash: false,
        }
        .endpoint()
        .expect("endpoint"),
        "items/top"
    );
    assert_eq!(
        ItemsRequest {
            collection: Some("ABCD1234".to_owned()),
            page: page(25),
            tag: None,
            top: true,
            trash: false,
        }
        .endpoint()
        .expect("endpoint"),
        "collections/ABCD1234/items/top"
    );
    assert_eq!(
        ItemsRequest {
            collection: None,
            page: page(25),
            tag: None,
            top: false,
            trash: true,
        }
        .endpoint()
        .expect("endpoint"),
        "items/trash"
    );
}

#[test]
fn rejects_invalid_items_request() {
    let err = ItemsRequest {
        collection: Some("ABCD1234".to_owned()),
        page: page(25),
        tag: None,
        top: false,
        trash: true,
    }
    .endpoint()
    .expect_err("invalid request");

    assert!(err.to_string().contains("--trash cannot be combined"));
}

#[test]
fn builds_items_tag_params() {
    assert_eq!(
        ItemsRequest {
            collection: None,
            page: page(25),
            tag: Some("neuroscience".to_owned()),
            top: false,
            trash: false,
        }
        .params(),
        vec![("tag", "neuroscience".to_owned())]
    );
}

#[test]
fn builds_page_params() {
    assert_eq!(
        PageRequest {
            limit: 25,
            start: 50,
            all: false,
            sort: Some("title".to_owned()),
            direction: Some("asc".to_owned()),
        }
        .params(vec![("tag", "neuroscience".to_owned())], 75),
        vec![
            ("tag", "neuroscience".to_owned()),
            ("limit", "25".to_owned()),
            ("start", "75".to_owned()),
            ("sort", "title".to_owned()),
            ("direction", "asc".to_owned()),
        ]
    );
}

#[test]
fn builds_export_requests() {
    let item_request = ExportRequest {
        format: "bibtex".to_owned(),
        item: Some("ABCD1234".to_owned()),
        collection: None,
        limit: 25,
        top: false,
        trash: false,
        style: None,
    };
    assert_eq!(item_request.endpoint().expect("endpoint"), "items/ABCD1234");
    assert_eq!(item_request.params(), vec![("format", "bibtex".to_owned())]);

    let collection_request = ExportRequest {
        format: "bib".to_owned(),
        item: None,
        collection: Some("COLL1234".to_owned()),
        limit: 50,
        top: true,
        trash: false,
        style: Some("apa".to_owned()),
    };
    assert_eq!(
        collection_request.endpoint().expect("endpoint"),
        "collections/COLL1234/items/top"
    );
    assert_eq!(
        collection_request.params(),
        vec![
            ("format", "bib".to_owned()),
            ("limit", "50".to_owned()),
            ("style", "apa".to_owned())
        ]
    );
}

#[test]
fn rejects_invalid_export_request() {
    let err = ExportRequest {
        format: "bibtex".to_owned(),
        item: Some("ABCD1234".to_owned()),
        collection: None,
        limit: 25,
        top: true,
        trash: false,
        style: None,
    }
    .endpoint()
    .expect_err("invalid request");

    assert!(err.to_string().contains("--item cannot be combined"));
}

#[test]
fn builds_item_children_endpoint() {
    assert_eq!(
        item_children_endpoint("ABCD1234"),
        "items/ABCD1234/children"
    );
}

#[test]
fn builds_tag_requests() {
    let request = TagsRequest {
        item: None,
        collection: Some("COLL1234".to_owned()),
        page: page(50),
        query: Some("neuro".to_owned()),
        qmode: "startsWith".to_owned(),
        top: true,
        trash: false,
    };

    assert_eq!(
        request.endpoint().expect("endpoint"),
        "collections/COLL1234/items/top/tags"
    );
    assert_eq!(
        request.params(),
        vec![
            ("qmode", "startsWith".to_owned()),
            ("q", "neuro".to_owned())
        ]
    );
}

#[test]
fn rejects_invalid_tags_request() {
    let err = TagsRequest {
        item: Some("ABCD1234".to_owned()),
        collection: None,
        page: page(50),
        query: None,
        qmode: "contains".to_owned(),
        top: true,
        trash: false,
    }
    .endpoint()
    .expect_err("invalid request");

    assert!(err.to_string().contains("--item cannot be combined"));
}

fn page(limit: u16) -> PageRequest {
    PageRequest {
        limit,
        start: 0,
        all: false,
        sort: None,
        direction: None,
    }
}
