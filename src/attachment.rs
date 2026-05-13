use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::{
    api::{AttachmentUpload, FileUploadResult, Item, LinkInfo, ZoteroClient},
    json_value::set_field,
};

pub(crate) async fn resolve_pdf_attachment(client: &ZoteroClient, key: &str) -> Result<Item> {
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

pub(crate) async fn build_attachment_item(
    client: &ZoteroClient,
    parent_key: &str,
    path: &Path,
    title: Option<&str>,
    content_type: &str,
) -> Result<Value> {
    let file = read_upload_file_metadata(path)?;
    let mut item = client.attachment_template().await?;
    set_field(&mut item, "parentItem", json!(parent_key));
    set_field(&mut item, "linkMode", json!("imported_file"));
    set_field(&mut item, "title", json!(title.unwrap_or(&file.filename)));
    set_field(&mut item, "filename", json!(file.filename));
    set_field(&mut item, "contentType", json!(content_type));
    Ok(item)
}

pub(crate) fn attach_to_json(upload: &FileUploadResult) -> Value {
    json!({
        "attachment_key": upload.attachment_key,
        "uploaded": upload.uploaded,
    })
}

pub(crate) async fn create_and_upload_attachment(
    client: &ZoteroClient,
    item: Value,
    path: &Path,
) -> Result<FileUploadResult> {
    let file = read_upload_file(path)?;
    let created = client.create_item(item).await?;
    let key = created.key().to_owned();

    match client
        .upload_attachment_file(&key, upload_request(&file))
        .await
    {
        Ok(upload) => Ok(upload),
        Err(upload_error) => {
            let cleanup = delete_created_attachment(client, &key).await;
            match cleanup {
                Ok(()) => Err(upload_error.context(
                    "attachment item was created, but file upload failed; removed the orphan attachment item",
                )),
                Err(cleanup_error) => Err(upload_error.context(format!(
                    "attachment item was created, but file upload failed; failed to remove orphan attachment {key}: {cleanup_error}"
                ))),
            }
        }
    }
}

async fn delete_created_attachment(client: &ZoteroClient, key: &str) -> Result<()> {
    let item = client.item(key).await?;
    client.delete_item(key, item.version).await?;
    Ok(())
}

pub(crate) struct UploadFile {
    pub(crate) filename: String,
    pub(crate) md5: String,
    pub(crate) filesize: u64,
    pub(crate) mtime_ms: i64,
    pub(crate) bytes: Vec<u8>,
}

struct UploadFileMetadata {
    filename: String,
    filesize: u64,
    mtime_ms: i64,
}

fn read_upload_file(path: &Path) -> Result<UploadFile> {
    let metadata = read_upload_file_metadata(path)?;
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let md5 = format!("{:x}", md5::compute(&bytes));

    Ok(UploadFile {
        filename: metadata.filename,
        md5,
        filesize: metadata.filesize,
        mtime_ms: metadata.mtime_ms,
        bytes,
    })
}

fn read_upload_file_metadata(path: &Path) -> Result<UploadFileMetadata> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    if !metadata.is_file() {
        bail!("attachment path is not a file: {}", path.display());
    }

    let filename = path
        .file_name()
        .and_then(|filename| filename.to_str())
        .with_context(|| format!("attachment path has no valid filename: {}", path.display()))?
        .to_owned();
    let mtime_ms = metadata
        .modified()
        .context("failed to read attachment modification time")?
        .duration_since(std::time::UNIX_EPOCH)
        .context("attachment modification time is before Unix epoch")?
        .as_millis() as i64;

    Ok(UploadFileMetadata {
        filename,
        filesize: metadata.len(),
        mtime_ms,
    })
}

pub(crate) async fn download_attachment(
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

    let bytes = read_attachment_bytes(client, &href).await?;
    let path = resolve_output_path(attachment, output)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    Ok(path)
}

async fn read_attachment_bytes(client: &ZoteroClient, href: &str) -> Result<Vec<u8>> {
    if let Some(path) = attachment_file_path(href)? {
        return fs::read(&path)
            .with_context(|| format!("failed to read attachment file {}", path.display()));
    }

    client.download_authenticated(href).await
}

pub(crate) fn attachment_file_path(href: &str) -> Result<Option<PathBuf>> {
    let Ok(url) = url::Url::parse(href) else {
        return Ok(None);
    };

    if url.scheme() != "file" {
        return Ok(None);
    }

    let path = url
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("invalid file attachment URL: {href}"))?;
    Ok(Some(path))
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

pub(crate) fn upload_request<'a>(file: &'a UploadFile) -> AttachmentUpload<'a> {
    AttachmentUpload {
        filename: &file.filename,
        md5: &file.md5,
        filesize: file.filesize,
        mtime_ms: file.mtime_ms,
        bytes: &file.bytes,
    }
}
