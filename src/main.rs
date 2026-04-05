mod api;
mod cli;
mod config;

use anyhow::Result;
use api::{Collection, Item, ZoteroClient};
use clap::Parser;
use cli::{Cli, Commands};
use config::{Config, resolve_user_id};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let Cli {
        profile,
        json,
        command,
    } = cli;

    match command {
        Commands::ResolveUser { username } => {
            let user_id = resolve_user_id(&username).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "username": username,
                        "user_id": user_id,
                    }))?
                );
            } else {
                println!("{username}  {user_id}");
            }
        }
        Commands::Search {
            query,
            limit,
            qmode,
            include_trashed,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let items = client
                .search_items(&query, limit, qmode.as_api_str(), include_trashed)
                .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                print_items(&items);
            }
        }
        Commands::Collections { query, limit, top } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let collections = client.collections(query.as_deref(), limit, top).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&collections)?);
            } else {
                print_collections(&collections);
            }
        }
        Commands::Item { key } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let item = client.item(&key).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&item)?);
            } else {
                print_item(&item);
            }
        }
    }

    Ok(())
}

fn print_items(items: &[Item]) {
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

fn print_collections(collections: &[Collection]) {
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

fn print_item(item: &Item) {
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
