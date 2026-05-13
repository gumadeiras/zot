use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::json_value::set_field;

pub(crate) struct UpdatePatchInput<'a> {
    pub title: Option<&'a str>,
    pub url: Option<&'a str>,
    pub tags: &'a [String],
    pub clear_tags: bool,
    pub collections: &'a [String],
    pub clear_collections: bool,
}

pub(crate) fn build_update_patch(input: UpdatePatchInput<'_>) -> Result<Value> {
    let mut patch = json!({});

    if let Some(title) = input.title {
        set_field(&mut patch, "title", json!(title));
    }

    if let Some(url) = input.url {
        set_field(&mut patch, "url", json!(url));
    }

    if input.clear_tags {
        set_field(&mut patch, "tags", json!([]));
    } else if !input.tags.is_empty() {
        let tags = input
            .tags
            .iter()
            .map(|tag| json!({ "tag": tag }))
            .collect::<Vec<_>>();
        set_field(&mut patch, "tags", Value::Array(tags));
    }

    if input.clear_collections {
        set_field(&mut patch, "collections", json!([]));
    } else if !input.collections.is_empty() {
        set_field(&mut patch, "collections", json!(input.collections));
    }

    if patch.as_object().is_none_or(serde_json::Map::is_empty) {
        bail!("no update fields provided");
    }

    Ok(patch)
}
