use super::*;
use crate::{
    add::{
        civil_from_days, csl_date_string, html_title, looks_like_inline_json,
        normalize_add_json_input, read_add_json_input,
    },
    attachment::attachment_file_path,
    update::{UpdatePatchInput, build_update_patch},
};
use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn extracts_html_title() {
    let html = "<html><head><title> Example Title </title></head></html>";
    assert_eq!(html_title(html).as_deref(), Some("Example Title"));
}

#[test]
fn formats_csl_date_parts() {
    let value = json!({ "date-parts": [[2024, 4, 5]] });
    assert_eq!(csl_date_string(&value).as_deref(), Some("2024-04-05"));
}

#[test]
fn converts_unix_days_to_date() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
}

#[test]
fn resolves_local_attachment_file_url() {
    let path = env::temp_dir().join("zot local attachment.pdf");
    let url = url::Url::from_file_path(&path).expect("file url");

    assert_eq!(
        attachment_file_path(url.as_str()).expect("path"),
        Some(path)
    );
    assert_eq!(
        attachment_file_path("https://api.zotero.org/users/1/items/ABC/file").expect("not file"),
        None
    );
}

#[test]
fn writes_export_output_to_nested_path() {
    let path = temp_test_path("exports/item.bib");

    write_export_output(&path, "@article{example}").expect("write export");
    assert_eq!(
        fs::read_to_string(&path).expect("read export"),
        "@article{example}"
    );

    fs::remove_file(&path).expect("cleanup file");
    fs::remove_dir(path.parent().expect("parent")).expect("cleanup dir");
}

#[test]
fn applies_add_collections_and_tags() {
    let mut item = json!({ "itemType": "webpage", "title": "Example" });

    apply_add_metadata(
        &mut item,
        &["COLL1234".to_owned()],
        &["neuroscience".to_owned()],
    );

    assert_eq!(item["collections"], json!(["COLL1234"]));
    assert_eq!(item["tags"], json!([{ "tag": "neuroscience" }]));
}

#[test]
fn merges_add_metadata_with_existing_json_fields() {
    let mut item = json!({
        "itemType": "webpage",
        "collections": ["EXISTING"],
        "tags": [{ "tag": "existing" }]
    });

    apply_add_metadata(
        &mut item,
        &["EXISTING".to_owned(), "NEW".to_owned()],
        &["existing".to_owned(), "new".to_owned()],
    );

    assert_eq!(item["collections"], json!(["EXISTING", "NEW"]));
    assert_eq!(
        item["tags"],
        json!([{ "tag": "existing" }, { "tag": "new" }])
    );
}

#[test]
fn builds_update_patch_from_fields() {
    let patch = build_update_patch(UpdatePatchInput {
        title: Some("New title"),
        url: Some("https://example.com/new"),
        tags: &["zot-live-test".to_owned()],
        clear_tags: false,
        collections: &[],
        clear_collections: true,
    })
    .expect("patch");

    assert_eq!(patch["title"], "New title");
    assert_eq!(patch["url"], "https://example.com/new");
    assert_eq!(patch["tags"], json!([{ "tag": "zot-live-test" }]));
    assert_eq!(patch["collections"], json!([]));
}

#[test]
fn rejects_empty_update_patch() {
    let err = build_update_patch(UpdatePatchInput {
        title: None,
        url: None,
        tags: &[],
        clear_tags: false,
        collections: &[],
        clear_collections: false,
    })
    .expect_err("empty patch");

    assert!(err.to_string().contains("no update fields"));
}

#[test]
fn normalizes_single_item_json_array() {
    let item = normalize_add_json_input(json!([{ "itemType": "webpage" }])).expect("item");
    assert_eq!(item["itemType"], "webpage");
}

#[test]
fn rejects_multi_item_json_array() {
    let err = normalize_add_json_input(json!([{ "itemType": "webpage" }, { "itemType": "book" }]))
        .expect_err("should reject multi-item arrays");
    assert!(err.to_string().contains("exactly one object"));
}

#[test]
fn detects_inline_json() {
    assert!(looks_like_inline_json("{\"itemType\":\"webpage\"}"));
    assert!(looks_like_inline_json("[{\"itemType\":\"webpage\"}]"));
    assert!(!looks_like_inline_json("item.json"));
}

#[test]
fn parses_inline_json_input() {
    let item = read_add_json_input(None, "{\"itemType\":\"webpage\"}").expect("item");
    assert_eq!(item["itemType"], "webpage");
}

#[test]
fn prefers_existing_file_path_over_inline_detection() {
    let path = temp_test_path("[draft].json");
    fs::write(&path, "{\"itemType\":\"webpage\",\"title\":\"From file\"}").expect("write");

    let item = read_add_json_input(None, path.to_str().expect("utf8 path")).expect("item");
    assert_eq!(item["title"], "From file");

    fs::remove_file(path).expect("cleanup");
}

#[test]
fn explicit_value_beats_file_path() {
    let path = temp_test_path("item.json");
    fs::write(&path, "{\"itemType\":\"webpage\",\"title\":\"From file\"}").expect("write");

    let item = read_add_json_input(
        Some("{\"itemType\":\"webpage\",\"title\":\"Inline\"}"),
        path.to_str().expect("utf8 path"),
    )
    .expect("item");
    assert_eq!(item["title"], "Inline");

    fs::remove_file(path).expect("cleanup");
}

fn temp_test_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    env::temp_dir().join(format!("zot-test-{nanos}-{name}"))
}
