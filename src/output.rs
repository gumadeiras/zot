use anyhow::Result;
use serde_json::{Value, json};
use std::{fs, path::Path};

use crate::api::{Collection, Group, Item, Tag, WriteSuccess};

pub fn print_items(items: &[Item]) {
    if items.is_empty() {
        println!("no items");
        return;
    }

    for item in items {
        println!(
            "{}  {}  {}",
            item.key,
            item.data.item_type,
            item.data.display_title()
        );

        let details = item.data.detail_line();
        if !details.is_empty() {
            println!("  {}", details);
        }
    }
}

pub fn print_collections(collections: &[Collection]) {
    if collections.is_empty() {
        println!("no collections");
        return;
    }

    for collection in collections {
        println!(
            "{}  {}",
            collection.key,
            collection.data.name.as_deref().unwrap_or("<unnamed>")
        );
        if let Some(parent) = &collection.data.parent_collection {
            println!("  parent {}", parent);
        }
    }
}

pub fn print_tags(tags: &[Tag]) {
    if tags.is_empty() {
        println!("no tags");
        return;
    }

    for tag in tags {
        println!("{}", tag.tag);
    }
}

pub fn print_groups(groups: &[Group]) {
    if groups.is_empty() {
        println!("no groups");
        return;
    }

    for group in groups {
        let name = group.data.name.as_deref().unwrap_or("<unnamed>");
        let group_type = group.data.group_type.as_deref().unwrap_or("group");
        println!("{}  {}  {}", group.id, group_type, name);
    }
}

pub fn print_item(item: &Item) {
    println!("key {}", item.key);
    println!("type {}", item.data.item_type);
    println!("title {}", item.data.display_title());

    let details = item.data.detail_line();
    if !details.is_empty() {
        println!("details {}", details);
    }

    if let Some(url) = &item.data.url {
        println!("url {}", url);
    }

    if let Some(parent) = &item.data.parent_item {
        println!("parent {}", parent);
    }
}

pub fn print_created_item(created: &WriteSuccess) {
    match created {
        WriteSuccess::Key(_) => println!("created {}", created.key()),
        WriteSuccess::Item(item) => {
            println!("created {}", created.key());
            println!("type {}", item.data.item_type);
            println!("title {}", item.data.display_title());
        }
    }
}

pub fn created_to_json(created: &WriteSuccess) -> Value {
    match created {
        WriteSuccess::Key(key) => json!({
            "key": key,
        }),
        WriteSuccess::Item(item) => json!({
            "key": item.key,
            "item": item,
        }),
    }
}

pub fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn write_export_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}
