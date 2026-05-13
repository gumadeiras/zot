use serde_json::{Value, json};

pub fn apply_add_metadata(item: &mut Value, collections: &[String], tags: &[String]) {
    if !collections.is_empty() {
        merge_string_array(item, "collections", collections);
    }

    if !tags.is_empty() {
        merge_tags(item, tags);
    }
}

fn merge_string_array(item: &mut Value, field: &str, additions: &[String]) {
    let mut values = item
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for addition in additions {
        if !values.iter().any(|value| value == addition) {
            values.push(addition.clone());
        }
    }

    set_field(item, field, json!(values));
}

fn merge_tags(item: &mut Value, additions: &[String]) {
    let mut values = item
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for addition in additions {
        if !values
            .iter()
            .any(|value| tag_name(value) == Some(addition.as_str()))
        {
            values.push(json!({ "tag": addition }));
        }
    }

    set_field(item, "tags", Value::Array(values));
}

fn tag_name(value: &Value) -> Option<&str> {
    value
        .get("tag")
        .and_then(Value::as_str)
        .or_else(|| value.as_str())
}

fn set_field(item: &mut Value, field: &str, value: Value) {
    if let Some(object) = item.as_object_mut() {
        object.insert(field.to_owned(), value);
    }
}
