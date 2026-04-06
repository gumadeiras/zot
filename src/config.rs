use anyhow::{Context, Result, bail};
use url::Url;

use crate::cli::ProfileArgs;

#[derive(Clone, Debug)]
pub enum LibraryScope {
    User(String),
    Group(String),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub api_base: Url,
    pub api_key: Option<String>,
    pub library: LibraryScope,
    pub local: bool,
}

impl Config {
    pub async fn from_profile(profile: &ProfileArgs) -> Result<Self> {
        if profile.local {
            return Ok(Self {
                api_base: Url::parse("http://localhost:23119/api")?,
                api_key: None,
                library: LibraryScope::User("0".to_owned()),
                local: true,
            });
        }

        let library = match (&profile.user_id, &profile.username, &profile.group_id) {
            (Some(user_id), None, None) => LibraryScope::User(user_id.clone()),
            (None, Some(username), None) => LibraryScope::User(resolve_user_id(username).await?),
            (None, None, Some(group_id)) => LibraryScope::Group(group_id.clone()),
            (None, None, None) => {
                bail!(
                    "missing library id; set --user-id, --username, --group-id, ZOTERO_USER_ID, ZOTERO_USERNAME, or ZOTERO_GROUP_ID"
                )
            }
            _ => bail!("pass only one of --user-id, --username, or --group-id"),
        };

        Ok(Self {
            api_base: Url::parse(&profile.api_base)?,
            api_key: profile.api_key.clone(),
            library,
            local: false,
        })
    }

    pub fn library_prefix(&self) -> String {
        match &self.library {
            LibraryScope::User(id) => format!("users/{id}"),
            LibraryScope::Group(id) => format!("groups/{id}"),
        }
    }
}

pub async fn resolve_user_id(username: &str) -> Result<String> {
    let profile_url = format!("https://www.zotero.org/{username}");
    let html = reqwest::get(&profile_url)
        .await
        .with_context(|| format!("request failed for {profile_url}"))?
        .error_for_status()
        .with_context(|| format!("profile lookup failed for {username}"))?
        .text()
        .await
        .with_context(|| format!("failed reading profile page for {username}"))?;

    extract_profile_user_id(&html)
        .with_context(|| format!("could not find profile user id for {username}"))
}

fn extract_profile_user_id(html: &str) -> Option<String> {
    let marker = "\"profileUserID\":";
    let start = html.find(marker)? + marker.len();
    let digits = html[start..]
        .chars()
        .skip_while(|ch| ch.is_ascii_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();

    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::cli::Cli;

    #[test]
    fn extracts_profile_id() {
        let html = r#"
            <script>
                window.zoteroData = {"profileUserID":6333263,"apiKey":""};
            </script>
        "#;
        assert_eq!(extract_profile_user_id(html).as_deref(), Some("6333263"));
    }

    #[test]
    fn rejects_missing_profile_id() {
        assert_eq!(extract_profile_user_id("<html></html>"), None);
    }

    #[tokio::test]
    async fn local_profile_defaults_to_users_zero() {
        let cli = Cli::parse_from(["zot", "--local", "collections"]);
        let config = Config::from_profile(&cli.profile).await.expect("config");
        assert!(config.local);
        assert!(matches!(config.library, LibraryScope::User(id) if id == "0"));
        assert_eq!(config.api_base.as_str(), "http://localhost:23119/api");
        assert!(config.api_key.is_none());
    }
}
