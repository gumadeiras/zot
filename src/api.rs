use anyhow::{Context, Result, bail};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use crate::config::Config;

const API_VERSION_HEADER: &str = "zotero-api-version";
const API_KEY_HEADER: &str = "zotero-api-key";
const WRITE_TOKEN_HEADER: &str = "zotero-write-token";

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
        limit: u16,
        qmode: &str,
        include_trashed: bool,
    ) -> Result<Vec<Item>> {
        let mut params = vec![
            ("q", query.to_owned()),
            ("qmode", qmode.to_owned()),
            ("limit", limit.to_string()),
            ("sort", "dateModified".to_owned()),
            ("direction", "desc".to_owned()),
        ];

        if include_trashed {
            params.push(("includeTrashed", "1".to_owned()));
        }

        self.get_json("items", &params).await
    }

    pub async fn collections(
        &self,
        query: Option<&str>,
        limit: u16,
        top: bool,
    ) -> Result<Vec<Collection>> {
        let endpoint = if top {
            "collections/top"
        } else {
            "collections"
        };
        let mut params = vec![("limit", limit.to_string())];

        if let Some(query) = query {
            params.push(("q", query.to_owned()));
        }

        self.get_json(endpoint, &params).await
    }

    pub async fn item(&self, key: &str) -> Result<Item> {
        self.get_json(&format!("items/{key}"), &[]).await
    }

    pub async fn item_children(&self, key: &str) -> Result<Vec<Item>> {
        self.get_json(&format!("items/{key}/children"), &[]).await
    }

    pub async fn item_template(&self, item_type: &str) -> Result<Value> {
        if self.config.local {
            bail!(
                "`zot add` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        self.get_json_absolute("items/new", &[("itemType", item_type.to_owned())])
            .await
    }

    pub async fn create_item(&self, item: Value) -> Result<WriteSuccess> {
        if self.config.local {
            bail!(
                "`zot add` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let url = self.library_url("items");
        let response = self
            .client
            .post(url)
            .header(
                HeaderName::from_static(WRITE_TOKEN_HEADER),
                Uuid::new_v4().simple().to_string(),
            )
            .json(&vec![item])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 403 && body.contains("Write access denied") {
                bail!(
                    "zotero api error {status}: {body}. create a write-enabled Zotero API key to use `zot add`"
                );
            }
            bail!("zotero api error {status}: {body}");
        }

        let body = response.json::<WriteResponse>().await?;
        if let Some(success) = body.successful.into_values().next() {
            return Ok(success);
        }

        if let Some(failure) = body.failed.into_values().next() {
            bail!("create failed {}: {}", failure.code, failure.message);
        }

        bail!("create request returned no successful or failed objects")
    }

    pub async fn download_authenticated(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| self.request_error(err))?;
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

        Ok(response.json::<T>().await?)
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
    Item(Item),
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
