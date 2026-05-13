use serde_json::Value;

pub(crate) fn set_field(item: &mut Value, field: &str, value: Value) {
    if let Some(object) = item.as_object_mut() {
        object.insert(field.to_owned(), value);
    }
}
