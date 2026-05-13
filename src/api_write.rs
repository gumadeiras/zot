use anyhow::{Context, Result, bail};
use reqwest::{
    Response,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, IF_NONE_MATCH},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{WRITE_TOKEN_HEADER, WriteResponse, WriteSuccess, ZoteroClient};

const IF_UNMODIFIED_SINCE_VERSION: &str = "if-unmodified-since-version";
const LAST_MODIFIED_VERSION: &str = "last-modified-version";

impl ZoteroClient {
    pub async fn attachment_template(&self) -> Result<Value> {
        self.item_template_with_params("attachment", &[("linkMode", "imported_file".to_owned())])
            .await
    }

    pub async fn item_template(&self, item_type: &str) -> Result<Value> {
        self.item_template_with_params(item_type, &[]).await
    }

    async fn item_template_with_params(
        &self,
        item_type: &str,
        extra_params: &[(&str, String)],
    ) -> Result<Value> {
        if self.config.local {
            bail!(
                "`zot add` and `zot attach` are not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let mut params = vec![("itemType", item_type.to_owned())];
        params.extend_from_slice(extra_params);
        self.get_json_absolute("items/new", &params).await
    }

    pub async fn create_item(&self, item: Value) -> Result<WriteSuccess> {
        if self.config.local {
            bail!(
                "`zot add` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let response = self
            .client
            .post(self.library_url("items"))
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

    pub async fn upload_attachment_file(
        &self,
        key: &str,
        file: AttachmentUpload<'_>,
    ) -> Result<FileUploadResult> {
        if self.config.local {
            bail!(
                "`zot attach` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let endpoint = format!("items/{key}/file");
        let auth = self
            .post_form(
                &endpoint,
                &[
                    ("md5", file.md5.to_owned()),
                    ("filename", file.filename.to_owned()),
                    ("filesize", file.filesize.to_string()),
                    ("mtime", file.mtime_ms.to_string()),
                ],
                Some("*"),
            )
            .await?
            .json::<UploadAuthorization>()
            .await?;

        if auth.exists == Some(1) {
            return Ok(FileUploadResult {
                attachment_key: key.to_owned(),
                uploaded: false,
            });
        }

        let url = auth
            .url
            .context("upload authorization did not include upload URL")?;
        let content_type = auth
            .content_type
            .context("upload authorization did not include content type")?;
        let upload_key = auth
            .upload_key
            .context("upload authorization did not include upload key")?;
        let prefix = auth.prefix.unwrap_or_default();
        let suffix = auth.suffix.unwrap_or_default();

        let mut body = Vec::with_capacity(prefix.len() + file.bytes.len() + suffix.len());
        body.extend_from_slice(prefix.as_bytes());
        body.extend_from_slice(file.bytes);
        body.extend_from_slice(suffix.as_bytes());

        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .body(body)
            .send()
            .await?;
        self.require_success(response).await?;

        self.post_form(&endpoint, &[("upload", upload_key)], Some("*"))
            .await?;

        Ok(FileUploadResult {
            attachment_key: key.to_owned(),
            uploaded: true,
        })
    }

    pub async fn update_item_patch(
        &self,
        key: &str,
        version: i64,
        patch: Value,
    ) -> Result<WriteActionResult> {
        if self.config.local {
            bail!(
                "`zot update` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let response = self
            .client
            .patch(self.library_url(&format!("items/{key}")))
            .header(
                HeaderName::from_static(IF_UNMODIFIED_SINCE_VERSION),
                version.to_string(),
            )
            .json(&patch)
            .send()
            .await?;
        let response = self.require_success(response).await?;

        Ok(WriteActionResult {
            key: key.to_owned(),
            version: modified_version(response.headers()),
        })
    }

    pub async fn delete_item(&self, key: &str, version: i64) -> Result<WriteActionResult> {
        if self.config.local {
            bail!(
                "`zot delete` is not supported in --local mode; use a write-enabled Zotero Web API key"
            );
        }

        let response = self
            .client
            .delete(self.library_url(&format!("items/{key}")))
            .header(
                HeaderName::from_static(IF_UNMODIFIED_SINCE_VERSION),
                version.to_string(),
            )
            .send()
            .await?;
        let response = self.require_success(response).await?;

        Ok(WriteActionResult {
            key: key.to_owned(),
            version: modified_version(response.headers()),
        })
    }

    async fn post_form(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
        if_none_match: Option<&str>,
    ) -> Result<Response> {
        let mut request = self.client.post(self.library_url(endpoint)).form(params);
        if let Some(if_none_match) = if_none_match {
            request = request.header(IF_NONE_MATCH, if_none_match);
        }
        let response = request.send().await?;
        self.require_success(response).await
    }
}

pub struct AttachmentUpload<'a> {
    pub filename: &'a str,
    pub md5: &'a str,
    pub filesize: u64,
    pub mtime_ms: i64,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Serialize)]
pub struct FileUploadResult {
    pub attachment_key: String,
    pub uploaded: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteActionResult {
    pub key: String,
    pub version: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadAuthorization {
    #[serde(default)]
    exists: Option<i64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    suffix: Option<String>,
    #[serde(default)]
    upload_key: Option<String>,
}

fn modified_version(headers: &HeaderMap) -> Option<i64> {
    headers
        .get(LAST_MODIFIED_VERSION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}
