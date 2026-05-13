mod add;
mod add_metadata;
mod api;
mod attachment;
mod cli;
mod config;
mod json_value;
mod output;
mod request;
mod update;

use std::process::Command;

use add::{build_add_item, read_add_json_input};
use add_metadata::apply_add_metadata;
use anyhow::{Context, Result, bail};
use api::{ExportRequest, ItemsRequest, PageRequest, TagsRequest, ZoteroClient};
use attachment::{
    attach_to_json, build_attachment_item, create_and_upload_attachment, download_attachment,
    resolve_pdf_attachment,
};
use clap::Parser;
use cli::{AddCommands, Cli, Commands};
use config::{Config, resolve_user_id};
use output::{
    created_to_json, print_collections, print_created_item, print_groups, print_item, print_items,
    print_json, print_tags, write_export_output,
};
use request::page_request;
use serde_json::json;
use update::{UpdatePatchInput, build_update_patch};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let Cli {
        profile,
        json,
        command,
    } = cli;

    match command {
        Commands::Items {
            collection,
            limit,
            start,
            all,
            sort,
            direction,
            tag,
            top,
            trash,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let items = client
                .items(ItemsRequest {
                    collection,
                    page: page_request(limit, start, all, sort, direction),
                    tag,
                    top,
                    trash,
                })
                .await?;
            if json {
                print_json(&items)?;
            } else {
                print_items(&items);
            }
        }
        Commands::Tags {
            item,
            collection,
            limit,
            query,
            qmode,
            top,
            trash,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let tags = client
                .tags(TagsRequest {
                    item,
                    collection,
                    page: page_request(limit, 0, false, None, None),
                    query,
                    qmode: qmode.as_api_str().to_owned(),
                    top,
                    trash,
                })
                .await?;
            if json {
                print_json(&tags)?;
            } else {
                print_tags(&tags);
            }
        }
        Commands::Children { key, limit } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let items = client.item_children_limited(&key, limit).await?;
            if json {
                print_json(&items)?;
            } else {
                print_items(&items);
            }
        }
        Commands::Groups { limit } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let groups = client.groups(limit).await?;
            if json {
                print_json(&groups)?;
            } else {
                print_groups(&groups);
            }
        }
        Commands::Open { key, zotero, print } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let item = client.item(&key).await?;
            let target = if zotero {
                item.links
                    .alternate
                    .as_ref()
                    .and_then(|link| link.href.clone())
                    .with_context(|| format!("no Zotero web URL for item {key}"))?
            } else {
                item.data
                    .url
                    .clone()
                    .or_else(|| {
                        item.links
                            .alternate
                            .as_ref()
                            .and_then(|link| link.href.clone())
                    })
                    .with_context(|| format!("no URL for item {key}"))?
            };

            if json {
                if !print {
                    open_target(&target)?;
                }
                print_json(&json!({
                    "key": key,
                    "target": target,
                    "opened": !print,
                    "source": if zotero { "zotero" } else { "item" },
                }))?;
            } else if print {
                println!("{target}");
            } else {
                open_target(&target)?;
            }
        }
        Commands::Pdf { key, output, print } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let attachment = resolve_pdf_attachment(&client, &key).await?;
            let path = download_attachment(&client, &attachment, output).await?;
            if json {
                if !print {
                    open_target(path.to_string_lossy().as_ref())?;
                }
                print_json(&json!({
                    "requested_key": key,
                    "attachment_key": attachment.key,
                    "path": path,
                    "opened": !print,
                }))?;
            } else if print {
                println!("{}", path.display());
            } else {
                open_target(path.to_string_lossy().as_ref())?;
            }
        }
        Commands::Attach {
            parent_key,
            path,
            title,
            content_type,
            dry_run,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let attachment =
                build_attachment_item(&client, &parent_key, &path, title.as_deref(), &content_type)
                    .await?;

            if dry_run {
                if json {
                    print_json(&json!({
                        "dry_run": true,
                        "item": attachment,
                    }))?;
                } else {
                    print_json(&attachment)?;
                }
            } else {
                let upload = create_and_upload_attachment(&client, attachment, &path).await?;
                if json {
                    print_json(&attach_to_json(&upload))?;
                } else {
                    println!("attached {}", upload.attachment_key);
                }
            }
        }
        Commands::Update {
            key,
            title,
            url,
            tag,
            clear_tags,
            collection,
            clear_collections,
            dry_run,
        } => {
            let patch = build_update_patch(UpdatePatchInput {
                title: title.as_deref(),
                url: url.as_deref(),
                tags: &tag,
                clear_tags,
                collections: &collection,
                clear_collections,
            })?;
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let item = client.item(&key).await?;

            if dry_run {
                print_json(&json!({
                    "dry_run": true,
                    "key": key,
                    "version": item.version,
                    "patch": patch,
                }))?;
            } else {
                let result = client.update_item_patch(&key, item.version, patch).await?;
                if json {
                    print_json(&result)?;
                } else {
                    println!("updated {}", result.key);
                }
            }
        }
        Commands::Delete { key, yes, dry_run } => {
            if !dry_run && !yes {
                bail!("refusing to delete {key} without --yes");
            }

            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let item = client.item(&key).await?;

            if dry_run {
                print_json(&json!({
                    "dry_run": true,
                    "key": key,
                    "version": item.version,
                }))?;
            } else {
                let result = client.delete_item(&key, item.version).await?;
                if json {
                    print_json(&result)?;
                } else {
                    println!("deleted {}", result.key);
                }
            }
        }
        Commands::Export {
            format,
            item,
            collection,
            limit,
            top,
            trash,
            output,
            style,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let format = format.as_api_str();
            let content = client
                .export(ExportRequest {
                    format: format.to_owned(),
                    item,
                    collection,
                    limit,
                    top,
                    trash,
                    style,
                })
                .await?;

            if let Some(path) = output {
                write_export_output(&path, &content)?;
                if json {
                    print_json(&json!({
                        "format": format,
                        "path": path,
                        "written": true,
                    }))?;
                } else {
                    println!("{}", path.display());
                }
            } else if json {
                print_json(&json!({
                    "format": format,
                    "content": content,
                }))?;
            } else {
                print!("{content}");
            }
        }
        Commands::Add {
            command,
            dry_run,
            collection,
            tag,
        } => {
            if dry_run {
                let mut item = match &command {
                    AddCommands::Json { value, input } => {
                        read_add_json_input(value.as_deref(), input)?
                    }
                    _ => {
                        let config = Config::from_profile(&profile).await?;
                        let client = ZoteroClient::new(config)?;
                        build_add_item(&client, &command).await?
                    }
                };
                apply_add_metadata(&mut item, &collection, &tag);

                if json {
                    print_json(&json!({
                        "dry_run": true,
                        "item": item,
                    }))?;
                } else {
                    print_json(&item)?;
                }
            } else {
                let config = Config::from_profile(&profile).await?;
                let client = ZoteroClient::new(config)?;
                let mut item = build_add_item(&client, &command).await?;
                apply_add_metadata(&mut item, &collection, &tag);
                let created = client.create_item(item).await?;
                if json {
                    print_json(&created_to_json(&created))?;
                } else {
                    print_created_item(&created);
                }
            }
        }
        Commands::ResolveUser { username } => {
            let user_id = resolve_user_id(&username).await?;
            if json {
                print_json(&serde_json::json!({
                    "username": username,
                    "user_id": user_id,
                }))?;
            } else {
                println!("{username}  {user_id}");
            }
        }
        Commands::Search {
            query,
            limit,
            start,
            all,
            sort,
            direction,
            qmode,
            include_trashed,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let items = client
                .search_items(
                    &query,
                    PageRequest {
                        limit,
                        start,
                        all,
                        sort: Some(sort.unwrap_or_else(|| "dateModified".to_owned())),
                        direction: Some(
                            direction
                                .map(|direction| direction.as_api_str().to_owned())
                                .unwrap_or_else(|| "desc".to_owned()),
                        ),
                    },
                    qmode.as_api_str(),
                    include_trashed,
                )
                .await?;
            if json {
                print_json(&items)?;
            } else {
                print_items(&items);
            }
        }
        Commands::Collections {
            query,
            limit,
            start,
            all,
            sort,
            direction,
            top,
        } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let collections = client
                .collections(
                    query.as_deref(),
                    page_request(limit, start, all, sort, direction),
                    top,
                )
                .await?;
            if json {
                print_json(&collections)?;
            } else {
                print_collections(&collections);
            }
        }
        Commands::Item { key } => {
            let config = Config::from_profile(&profile).await?;
            let client = ZoteroClient::new(config)?;
            let item = client.item(&key).await?;
            if json {
                print_json(&item)?;
            } else {
                print_item(&item);
            }
        }
    }

    Ok(())
}

fn open_target(target: &str) -> Result<()> {
    let status = Command::new("open")
        .arg(target)
        .status()
        .with_context(|| format!("failed to run open for {target}"))?;
    if !status.success() {
        bail!("open exited with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
