use std::{fs, io::Read, path::Path};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::{Value, json};

use crate::{api::ZoteroClient, cli::AddCommands, json_value::set_field};

pub(crate) async fn build_add_item(client: &ZoteroClient, command: &AddCommands) -> Result<Value> {
    match command {
        AddCommands::Json { value, input } => read_add_json_input(value.as_deref(), input),
        AddCommands::Doi { doi } => build_doi_item(client, doi).await,
        AddCommands::Isbn { isbn } => build_isbn_item(client, isbn).await,
        AddCommands::Url { url, title } => build_url_item(client, url, title.as_deref()).await,
    }
}

pub(crate) fn read_add_json_input(value: Option<&str>, input: &str) -> Result<Value> {
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

pub(crate) fn looks_like_inline_json(input: &str) -> bool {
    let trimmed = input.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

pub(crate) fn normalize_add_json_input(value: Value) -> Result<Value> {
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
    if let Ok(parsed) = url::Url::parse(url)
        && let Some(host) = parsed.host_str()
    {
        set_field(&mut item, "websiteTitle", json!(host));
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

    html_title(&html).with_context(|| format!("no <title> found at {url}"))
}

pub(crate) fn html_title(html: &str) -> Option<String> {
    let title_regex = Regex::new("(?is)<title[^>]*>(.*?)</title>").expect("valid regex");
    title_regex
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|title| title.as_str().trim().replace('\n', " "))
        .filter(|title| !title.is_empty())
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

pub(crate) fn csl_date_string(value: &Value) -> Option<String> {
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

pub(crate) fn today_utc_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs() as i64;
    let days = now.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

pub(crate) fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
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
