use anyhow::{Context, Result, bail};
use reqwest::{
    Client, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::config::{Config, LibraryScope};

const API_VERSION_HEADER: &str = "zotero-api-version";
const API_KEY_HEADER: &str = "zotero-api-key";
const WRITE_TOKEN_HEADER: &str = "zotero-write-token";

#[path = "api_write.rs"]
mod write;
pub use write::{AttachmentUpload, FileUploadResult};

#[derive(Clone)]
pub struct ZoteroClient {
    client: Client,
    config: Config,
}

impl ZoteroClient {
    pub fn new(config: Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(API_VERSION_HEADER),
            HeaderValue::from_static("3"),
        );

        if let Some(api_key) = &config.api_key {
            headers.insert(
                HeaderName::from_static(API_KEY_HEADER),
                HeaderValue::from_str(api_key).context("invalid api key header value")?,
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .user_agent(concat!("zot/", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self { client, config })
    }

    pub async fn search_items(
        &self,
        query: &str,
        page: PageRequest,
        qmode: &str,
        include_trashed: bool,
    ) -> Result<Vec<Item>> {
        let mut params = vec![("q", query.to_owned()), ("qmode", qmode.to_owned())];

        if include_trashed {
            params.push(("includeTrashed", "1".to_owned()));
        }

        self.get_json_list("items", params, &page).await
    }

    pub async fn items(&self, request: ItemsRequest) -> Result<Vec<Item>> {
        let endpoint = request.endpoint()?;
        let params = request.params();
        self.get_json_list(&endpoint, params, &request.page).await
    }

    pub async fn tags(&self, request: TagsRequest) -> Result<Vec<Tag>> {
        let endpoint = request.endpoint()?;
        let params = request.params();
        self.get_json_list(&endpoint, params, &request.page).await
    }

    pub async fn export(&self, request: ExportRequest) -> Result<String> {
        let endpoint = request.endpoint()?;
        let params = request.params();
        self.get_text(&endpoint, &params).await
    }

    pub async fn collections(
        &self,
        query: Option<&str>,
        page: PageRequest,
        top: bool,
    ) -> Result<Vec<Collection>> {
        let endpoint = if top {
            "collections/top"
        } else {
            "collections"
        };
        let mut params = Vec::new();

        if let Some(query) = query {
            params.push(("q", query.to_owned()));
        }

        self.get_json_list(endpoint, params, &page).await
    }

    pub async fn groups(&self, limit: u16) -> Result<Vec<Group>> {
        let LibraryScope::User(user_id) = &self.config.library else {
            bail!("`zot groups` requires a user library; pass --user-id or --username");
        };

        if self.config.local {
            bail!("`zot groups` is not supported in --local mode; pass --user-id or --username");
        }

        self.get_json_absolute(
            &format!("users/{user_id}/groups"),
            &[("limit", limit.to_string())],
        )
        .await
    }

    pub async fn item(&self, key: &str) -> Result<Item> {
        self.get_json(&format!("items/{key}"), &[]).await
    }

    pub async fn item_children(&self, key: &str) -> Result<Vec<Item>> {
        self.get_json(&item_children_endpoint(key), &[]).await
    }

    pub async fn item_children_limited(&self, key: &str, limit: u16) -> Result<Vec<Item>> {
        self.get_json(
            &item_children_endpoint(key),
            &[("limit", limit.to_string())],
        )
        .await
    }

    pub async fn download_authenticated(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| self.request_error(err))?;
        let response = self.require_success(response).await?;

        Ok(response.bytes().await?.to_vec())
    }

    async fn get_json<T>(&self, endpoint: &str, query: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.library_url(endpoint);
        self.get_json_from_url(&url, query).await
    }

    async fn get_json_absolute<T>(&self, endpoint: &str, query: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "{}/{}",
            self.config.api_base.as_str().trim_end_matches('/'),
            endpoint
        );
        self.get_json_from_url(&url, query).await
    }

    async fn get_json_from_url<T>(&self, url: &str, query: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .client
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|err| self.request_error(err))?;
        let response = self.require_success(response).await?;

        Ok(response.json::<T>().await?)
    }

    async fn get_text(&self, endpoint: &str, query: &[(&str, String)]) -> Result<String> {
        let url = self.library_url(endpoint);
        let response = self
            .client
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|err| self.request_error(err))?;
        let response = self.require_success(response).await?;

        Ok(response.text().await?)
    }

    async fn get_json_list<T>(
        &self,
        endpoint: &str,
        params: Vec<(&str, String)>,
        page: &PageRequest,
    ) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        if !page.all {
            let params = page.params(params, page.start);
            return self.get_json(endpoint, &params).await;
        }

        let mut all = Vec::new();
        let mut start = page.start;

        loop {
            let params = page.params(params.clone(), start);
            let mut chunk = self.get_json::<Vec<T>>(endpoint, &params).await?;
            let chunk_len = chunk.len();
            all.append(&mut chunk);

            if chunk_len < page.limit as usize {
                break;
            }

            start += chunk_len as u32;
        }

        Ok(all)
    }

    async fn require_success(&self, response: Response) -> Result<Response> {
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if self.config.local && body.contains("Local API is not enabled") {
                bail!(
                    "local Zotero API is disabled at {}. enable 'Allow other applications on this computer to communicate with Zotero'",
                    self.config.api_base
                );
            }
            bail!("zotero api error {status}: {body}");
        }

        Ok(response)
    }

    fn library_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}/{}",
            self.config.api_base.as_str().trim_end_matches('/'),
            self.config.library_prefix(),
            endpoint
        )
    }

    fn request_error(&self, err: reqwest::Error) -> anyhow::Error {
        if self.config.local && err.is_connect() {
            return anyhow::anyhow!(
                "failed to connect to the local Zotero API at {}. start Zotero and enable 'Allow other applications on this computer to communicate with Zotero'",
                self.config.api_base
            );
        }

        err.into()
    }
}

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub limit: u16,
    pub start: u32,
    pub all: bool,
    pub sort: Option<String>,
    pub direction: Option<String>,
}

impl PageRequest {
    fn params<'a>(
        &'a self,
        mut params: Vec<(&'a str, String)>,
        start: u32,
    ) -> Vec<(&'a str, String)> {
        params.push(("limit", self.limit.to_string()));
        params.push(("start", start.to_string()));

        if let Some(sort) = &self.sort {
            params.push(("sort", sort.clone()));
        }

        if let Some(direction) = &self.direction {
            params.push(("direction", direction.clone()));
        }

        params
    }
}

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub format: String,
    pub item: Option<String>,
    pub collection: Option<String>,
    pub limit: u16,
    pub top: bool,
    pub trash: bool,
    pub style: Option<String>,
}

impl ExportRequest {
    fn endpoint(&self) -> Result<String> {
        if self.item.is_some() && (self.collection.is_some() || self.top || self.trash) {
            bail!("--item cannot be combined with --collection, --top, or --trash");
        }

        if let Some(item) = &self.item {
            return Ok(format!("items/{item}"));
        }

        item_list_endpoint(self.collection.as_deref(), self.top, self.trash)
    }

    fn params(&self) -> Vec<(&str, String)> {
        let mut params = vec![("format", self.format.clone())];

        if self.item.is_none() {
            params.push(("limit", self.limit.to_string()));
        }

        if let Some(style) = &self.style {
            params.push(("style", style.clone()));
        }

        params
    }
}

#[derive(Debug, Clone)]
pub struct ItemsRequest {
    pub collection: Option<String>,
    pub page: PageRequest,
    pub tag: Option<String>,
    pub top: bool,
    pub trash: bool,
}

impl ItemsRequest {
    fn endpoint(&self) -> Result<String> {
        item_list_endpoint(self.collection.as_deref(), self.top, self.trash)
    }

    fn params(&self) -> Vec<(&str, String)> {
        let mut params = Vec::new();

        if let Some(tag) = &self.tag {
            params.push(("tag", tag.clone()));
        }

        params
    }
}

#[derive(Debug, Clone)]
pub struct TagsRequest {
    pub item: Option<String>,
    pub collection: Option<String>,
    pub page: PageRequest,
    pub query: Option<String>,
    pub qmode: String,
    pub top: bool,
    pub trash: bool,
}

impl TagsRequest {
    fn endpoint(&self) -> Result<String> {
        if self.item.is_some() && (self.collection.is_some() || self.top || self.trash) {
            bail!("--item cannot be combined with --collection, --top, or --trash");
        }

        if let Some(item) = &self.item {
            return Ok(format!("items/{item}/tags"));
        }

        item_tags_endpoint(self.collection.as_deref(), self.top, self.trash)
    }

    fn params(&self) -> Vec<(&str, String)> {
        let mut params = vec![("qmode", self.qmode.clone())];

        if let Some(query) = &self.query {
            params.push(("q", query.clone()));
        }

        params
    }
}

fn item_list_endpoint(collection: Option<&str>, top: bool, trash: bool) -> Result<String> {
    if trash {
        if collection.is_some() || top {
            bail!("--trash cannot be combined with --collection or --top");
        }
        return Ok("items/trash".to_owned());
    }

    match (collection, top) {
        (Some(collection), true) => Ok(format!("collections/{collection}/items/top")),
        (Some(collection), false) => Ok(format!("collections/{collection}/items")),
        (None, true) => Ok("items/top".to_owned()),
        (None, false) => Ok("items".to_owned()),
    }
}

fn item_tags_endpoint(collection: Option<&str>, top: bool, trash: bool) -> Result<String> {
    if trash {
        if collection.is_some() || top {
            bail!("--trash cannot be combined with --collection or --top");
        }
        return Ok("items/trash/tags".to_owned());
    }

    match (collection, top) {
        (Some(collection), true) => Ok(format!("collections/{collection}/items/top/tags")),
        (Some(collection), false) => Ok(format!("collections/{collection}/items/tags")),
        (None, true) => Ok("items/top/tags".to_owned()),
        (None, false) => Ok("tags".to_owned()),
    }
}

fn item_children_endpoint(key: &str) -> String {
    format!("items/{key}/children")
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Item {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub links: ItemLinks,
    pub data: ItemData,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Tag {
    #[serde(default)]
    pub tag: String,
    #[serde(default, rename = "type")]
    pub tag_type: Option<i64>,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Group {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub data: GroupData,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub links: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "type")]
    pub group_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemData {
    #[serde(default)]
    pub item_type: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub short_title: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub url: Option<String>,
    #[serde(
        default,
        alias = "DOI",
        deserialize_with = "deserialize_option_stringish"
    )]
    pub doi: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub filename: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub content_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub parent_item: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub publication_title: Option<String>,
    #[serde(
        default,
        alias = "ISBN",
        deserialize_with = "deserialize_option_stringish"
    )]
    pub isbn: Option<String>,
    #[serde(default)]
    pub creators: Vec<Creator>,
}

impl ItemData {
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .or(self.subject.as_deref())
            .or(self.filename.as_deref())
            .or(self.short_title.as_deref())
            .unwrap_or("<untitled>")
    }

    pub fn detail_line(&self) -> String {
        let mut bits = Vec::new();

        if let Some(creators) = creator_summary(&self.creators) {
            bits.push(creators);
        }
        if let Some(date) = &self.date {
            bits.push(date.clone());
        }
        if let Some(pub_title) = &self.publication_title {
            bits.push(pub_title.clone());
        }
        if let Some(doi) = &self.doi {
            bits.push(format!("doi:{doi}"));
        }
        if let Some(isbn) = &self.isbn {
            bits.push(format!("isbn:{isbn}"));
        }
        if let Some(content_type) = &self.content_type {
            bits.push(content_type.clone());
        }

        bits.join(" | ")
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Collection {
    pub key: String,
    pub version: i64,
    pub data: CollectionData,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ItemLinks {
    #[serde(default)]
    pub alternate: Option<LinkInfo>,
    #[serde(default)]
    pub enclosure: Option<LinkInfo>,
    #[serde(default)]
    pub attachment: Option<LinkInfo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LinkInfo {
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub length: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_stringish")]
    pub parent_collection: Option<String>,
}

fn creator_summary(creators: &[Creator]) -> Option<String> {
    let names = creators
        .iter()
        .filter_map(|creator| {
            creator.name.clone().or_else(|| {
                match (creator.first_name.as_deref(), creator.last_name.as_deref()) {
                    (Some(first), Some(last)) => Some(format!("{first} {last}")),
                    (None, Some(last)) => Some(last.to_owned()),
                    (Some(first), None) => Some(first.to_owned()),
                    (None, None) => None,
                }
            })
        })
        .take(3)
        .collect::<Vec<_>>();

    if names.is_empty() {
        None
    } else {
        let suffix = if creators.len() > names.len() {
            " et al."
        } else {
            ""
        };
        Some(format!("{}{}", names.join(", "), suffix))
    }
}

fn deserialize_option_stringish<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stringish {
        String(String),
        Bool(bool),
        Number(serde_json::Number),
        Null,
    }

    match Option::<Stringish>::deserialize(deserializer)? {
        Some(Stringish::String(value)) => Ok(Some(value)),
        Some(Stringish::Number(value)) => Ok(Some(value.to_string())),
        Some(Stringish::Bool(false)) | Some(Stringish::Null) | None => Ok(None),
        Some(Stringish::Bool(true)) => Ok(Some("true".to_owned())),
    }
}

#[derive(Debug, Deserialize)]
pub struct WriteResponse {
    #[serde(default, alias = "success")]
    pub successful: std::collections::BTreeMap<String, WriteSuccess>,
    #[serde(default)]
    pub failed: std::collections::BTreeMap<String, WriteFailure>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WriteSuccess {
    Key(String),
    Item(Box<Item>),
}

impl WriteSuccess {
    pub fn key(&self) -> &str {
        match self {
            Self::Key(key) => key,
            Self::Item(item) => &item.key,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WriteFailure {
    #[serde(default)]
    pub code: u16,
    #[serde(default)]
    pub message: String,
}
