mod api;
mod cli;
mod config;

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use api::{Collection, Item, LinkInfo, WriteSuccess, ZoteroClient};
use clap::Parser;
use cli::{AddCommands, Cli, Commands};
use config::{Config, resolve_user_id};
use regex::Regex;
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let Cli {
        profile,
        json,
        command,
    } = cli;

    match command {
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
        Commands::Add { command, dry_run } => {
            if dry_run {
                let item = match &command {
                    AddCommands::Json { value, input } => {
                        read_add_json_input(value.as_deref(), input)?
                    }
                    _ => {
                        let config = Config::from_profile(&profile).await?;
                        let client = ZoteroClient::new(config)?;
                        build_add_item(&client, &command).await?
                    }
                };

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
                let item = build_add_item(&client, &command).await?;
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

fn print_created_item(created: &WriteSuccess) {
    match created {
        WriteSuccess::Key(_) => println!("created {}", created.key()),
        WriteSuccess::Item(item) => {
            println!("created {}", created.key());
            println!("type {}", item.data.item_type);
            println!("title {}", item.data.display_title());
        }
    }
}

fn created_to_json(created: &WriteSuccess) -> Value {
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

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

async fn resolve_pdf_attachment(client: &ZoteroClient, key: &str) -> Result<Item> {
    let item = client.item(key).await?;
    if is_pdf_attachment(&item) {
        return Ok(item);
    }

    client
        .item_children(key)
        .await?
        .into_iter()
        .find(is_pdf_attachment)
        .with_context(|| format!("no PDF attachment found for item {key}"))
}

fn is_pdf_attachment(item: &Item) -> bool {
    item.data.item_type == "attachment"
        && (item.data.content_type.as_deref() == Some("application/pdf")
            || item.links.enclosure.as_ref().and_then(link_type) == Some("application/pdf"))
}

fn link_type(link: &LinkInfo) -> Option<&str> {
    link.r#type.as_deref()
}

async fn download_attachment(
    client: &ZoteroClient,
    attachment: &Item,
    output: Option<PathBuf>,
) -> Result<PathBuf> {
    let href = attachment
        .links
        .enclosure
        .as_ref()
        .and_then(|link| link.href.clone())
        .with_context(|| format!("attachment {} has no download URL", attachment.key))?;

    let bytes = client.download_authenticated(&href).await?;
    let path = resolve_output_path(attachment, output)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    Ok(path)
}

fn resolve_output_path(attachment: &Item, output: Option<PathBuf>) -> Result<PathBuf> {
    let filename = sanitize_filename(
        attachment
            .data
            .filename
            .as_deref()
            .unwrap_or("attachment.pdf"),
    );

    match output {
        Some(path) if path.is_dir() => Ok(path.join(filename)),
        Some(path) => Ok(path),
        None => Ok(std::env::temp_dir().join(format!("zot-{}-{}", attachment.key, filename))),
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '\0') {
                '-'
            } else {
                ch
            }
        })
        .collect()
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

async fn build_add_item(client: &ZoteroClient, command: &AddCommands) -> Result<Value> {
    match command {
        AddCommands::Json { value, input } => read_add_json_input(value.as_deref(), input),
        AddCommands::Doi { doi } => build_doi_item(client, doi).await,
        AddCommands::Isbn { isbn } => build_isbn_item(client, isbn).await,
        AddCommands::Url { url, title } => build_url_item(client, url, title.as_deref()).await,
    }
}

fn read_add_json_input(value: Option<&str>, input: &str) -> Result<Value> {
    let source = resolve_add_json_source(value, input);
    let raw = read_add_json_raw(&source)?;
    let value =
        serde_json::from_str::<Value>(&raw).with_context(|| add_json_parse_error(&source))?;

    normalize_add_json_input(value)
}

enum AddJsonSource<'a> {
    Inline(&'a str),
    Stdin,
    File(&'a Path),
}

fn resolve_add_json_source<'a>(value: Option<&'a str>, input: &'a str) -> AddJsonSource<'a> {
    if let Some(value) = value {
        return AddJsonSource::Inline(value);
    }

    if input == "-" {
        return AddJsonSource::Stdin;
    }

    let path = Path::new(input);
    if path.exists() {
        return AddJsonSource::File(path);
    }

    if looks_like_inline_json(input) {
        return AddJsonSource::Inline(input);
    }

    AddJsonSource::File(path)
}

fn read_add_json_raw(source: &AddJsonSource<'_>) -> Result<String> {
    match source {
        AddJsonSource::Inline(value) => Ok((*value).to_owned()),
        AddJsonSource::Stdin => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .context("failed to read JSON from stdin")?;
            Ok(buffer)
        }
        AddJsonSource::File(path) => fs::read_to_string(path)
            .with_context(|| format!("failed to read JSON from {}", path.display())),
    }
}

fn add_json_parse_error(source: &AddJsonSource<'_>) -> String {
    match source {
        AddJsonSource::Inline(_) => "failed to parse inline JSON input".to_owned(),
        AddJsonSource::Stdin => "failed to parse JSON from stdin".to_owned(),
        AddJsonSource::File(path) => format!("failed to parse JSON from {}", path.display()),
    }
}

fn looks_like_inline_json(input: &str) -> bool {
    let trimmed = input.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn normalize_add_json_input(value: Value) -> Result<Value> {
    match value {
        Value::Object(_) => Ok(value),
        Value::Array(mut items) if items.len() == 1 => {
            let item = items.pop().expect("single-element array");
            if item.is_object() {
                Ok(item)
            } else {
                bail!("JSON array input must contain exactly one object item")
            }
        }
        Value::Array(_) => bail!("JSON array input must contain exactly one object item"),
        _ => bail!("JSON input must be a Zotero item object or a single-item array"),
    }
}

async fn build_doi_item(client: &ZoteroClient, doi: &str) -> Result<Value> {
    let mut item = client.item_template("journalArticle").await?;
    let doi = doi.trim();
    set_field(&mut item, "creators", json!([]));
    set_field(&mut item, "DOI", json!(doi));
    set_field(&mut item, "url", json!(format!("https://doi.org/{doi}")));

    if let Ok(metadata) = fetch_doi_metadata(doi).await {
        apply_csl_metadata(&mut item, metadata);
    } else {
        set_field(&mut item, "title", json!(format!("DOI {doi}")));
    }

    Ok(item)
}

async fn build_isbn_item(client: &ZoteroClient, isbn: &str) -> Result<Value> {
    let mut item = client.item_template("book").await?;
    let isbn = isbn.trim();
    set_field(&mut item, "creators", json!([]));
    set_field(&mut item, "ISBN", json!(isbn));
    set_field(&mut item, "title", json!(format!("ISBN {isbn}")));
    Ok(item)
}

async fn build_url_item(client: &ZoteroClient, url: &str, title: Option<&str>) -> Result<Value> {
    let mut item = client.item_template("webpage").await?;
    let page_title = match title {
        Some(title) => title.to_owned(),
        None => fetch_html_title(url)
            .await
            .unwrap_or_else(|_| url.to_owned()),
    };

    set_field(&mut item, "creators", json!([]));
    set_field(&mut item, "title", json!(page_title));
    set_field(&mut item, "url", json!(url));
    set_field(&mut item, "accessDate", json!(today_utc_date()));
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            set_field(&mut item, "websiteTitle", json!(host));
        }
    }

    Ok(item)
}

async fn fetch_html_title(url: &str) -> Result<String> {
    let html = reqwest::Client::new()
        .get(url)
        .header("User-Agent", concat!("zot/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let title_regex = Regex::new("(?is)<title[^>]*>(.*?)</title>").expect("valid regex");
    let title = title_regex
        .captures(&html)
        .and_then(|captures| captures.get(1))
        .map(|title| title.as_str().trim().replace('\n', " "))
        .filter(|title| !title.is_empty())
        .with_context(|| format!("no <title> found at {url}"))?;

    Ok(title)
}

async fn fetch_doi_metadata(doi: &str) -> Result<Value> {
    let response = reqwest::Client::new()
        .get(format!("https://doi.org/{doi}"))
        .header("Accept", "application/vnd.citationstyles.csl+json")
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

fn apply_csl_metadata(item: &mut Value, metadata: Value) {
    if let Some(title) = metadata.get("title").and_then(as_string) {
        set_field(item, "title", json!(title));
    }
    if let Some(url) = metadata.get("URL").and_then(as_string) {
        set_field(item, "url", json!(url));
    }
    if let Some(container) = metadata
        .get("container-title")
        .and_then(csl_string_or_first_array)
    {
        set_field(item, "publicationTitle", json!(container));
    }
    if let Some(date) = metadata.get("issued").and_then(csl_date_string) {
        set_field(item, "date", json!(date));
    }
    if let Some(authors) = metadata.get("author").and_then(csl_authors) {
        set_field(item, "creators", Value::Array(authors));
    } else {
        set_field(item, "creators", json!([]));
    }
}

fn as_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn csl_string_or_first_array(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_owned());
    }

    value
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn csl_date_string(value: &Value) -> Option<String> {
    let date_parts = value.get("date-parts")?.as_array()?.first()?.as_array()?;
    let mut parts = date_parts
        .iter()
        .filter_map(|part| part.as_i64())
        .map(|part| part.to_string())
        .collect::<Vec<_>>();

    if parts.len() >= 2 && parts[1].len() == 1 {
        parts[1] = format!("0{}", parts[1]);
    }
    if parts.len() >= 3 && parts[2].len() == 1 {
        parts[2] = format!("0{}", parts[2]);
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("-"))
    }
}

fn csl_authors(value: &Value) -> Option<Vec<Value>> {
    let authors = value.as_array()?;
    let creators = authors
        .iter()
        .map(|author| {
            if let Some(literal) = author.get("literal").and_then(Value::as_str) {
                json!({
                    "creatorType": "author",
                    "name": literal,
                })
            } else {
                json!({
                    "creatorType": "author",
                    "firstName": author.get("given").and_then(Value::as_str).unwrap_or_default(),
                    "lastName": author.get("family").and_then(Value::as_str).unwrap_or_default(),
                })
            }
        })
        .collect::<Vec<_>>();

    Some(creators)
}

fn set_field(item: &mut Value, field: &str, value: Value) {
    if let Some(object) = item.as_object_mut() {
        object.insert(field.to_owned(), value);
    }
}

fn today_utc_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs() as i64;
    let days = now.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn extracts_html_title() {
        let html = "<html><head><title> Example Title </title></head></html>";
        let regex = Regex::new("(?is)<title[^>]*>(.*?)</title>").expect("valid regex");
        let title = regex
            .captures(html)
            .and_then(|captures| captures.get(1))
            .map(|title| title.as_str().trim().replace('\n', " "));
        assert_eq!(title.as_deref(), Some("Example Title"));
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
    fn normalizes_single_item_json_array() {
        let item = normalize_add_json_input(json!([{ "itemType": "webpage" }])).expect("item");
        assert_eq!(item["itemType"], "webpage");
    }

    #[test]
    fn rejects_multi_item_json_array() {
        let err =
            normalize_add_json_input(json!([{ "itemType": "webpage" }, { "itemType": "book" }]))
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
}
