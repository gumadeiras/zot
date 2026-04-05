use anyhow::{Context, Result, bail};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::config::Config;

const API_VERSION_HEADER: &str = "zotero-api-version";
const API_KEY_HEADER: &str = "zotero-api-key";

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

    async fn get_json<T>(&self, endpoint: &str, query: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "{}/{}/{}",
            self.config.api_base.as_str().trim_end_matches('/'),
            self.config.library_prefix(),
            endpoint
        );

        let response = self.client.get(url).query(query).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("zotero api error {status}: {body}");
        }

        Ok(response.json::<T>().await?)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Item {
    pub key: String,
    pub version: i64,
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
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub doi: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub parent_item: Option<String>,
    #[serde(default)]
    pub publication_title: Option<String>,
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
#[serde(rename_all = "camelCase")]
pub struct CollectionData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
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
