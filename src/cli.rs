use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "zot", version, about = "Small Zotero CLI")]
pub struct Cli {
    #[command(flatten)]
    pub profile: ProfileArgs,

    #[arg(long, global = true, help = "Emit raw JSON")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[arg(
        long,
        env = "ZOTERO_LOCAL",
        global = true,
        conflicts_with_all = ["user_id", "username", "group_id", "api_key"],
        help = "Use the local Zotero desktop API at http://localhost:23119/api"
    )]
    pub local: bool,

    #[arg(
        long,
        env = "ZOTERO_USER_ID",
        global = true,
        conflicts_with_all = ["group_id", "username", "local"],
        help = "Zotero user id"
    )]
    pub user_id: Option<String>,

    #[arg(
        long,
        env = "ZOTERO_USERNAME",
        global = true,
        conflicts_with_all = ["user_id", "group_id", "local"],
        help = "Zotero username; resolves numeric user id from profile page"
    )]
    pub username: Option<String>,

    #[arg(
        long,
        env = "ZOTERO_GROUP_ID",
        global = true,
        conflicts_with_all = ["user_id", "username", "local"],
        help = "Zotero group id"
    )]
    pub group_id: Option<String>,

    #[arg(long, env = "ZOTERO_API_KEY", global = true, help = "Zotero API key")]
    pub api_key: Option<String>,

    #[arg(
        long,
        env = "ZOTERO_API_BASE",
        global = true,
        default_value = "https://api.zotero.org",
        help = "API base URL"
    )]
    pub api_base: String,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Items {
        #[arg(long)]
        collection: Option<String>,
        #[arg(long, default_value_t = 25, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
        #[arg(long, default_value_t = 0)]
        start: u32,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long, value_enum)]
        direction: Option<SortDirection>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        top: bool,
        #[arg(long, conflicts_with_all = ["collection", "top"])]
        trash: bool,
    },
    Tags {
        #[arg(long, conflicts_with_all = ["collection", "top", "trash"])]
        item: Option<String>,
        #[arg(long, conflicts_with = "trash")]
        collection: Option<String>,
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, value_enum, default_value_t = TagSearchMode::Contains)]
        qmode: TagSearchMode,
        #[arg(long, conflicts_with_all = ["item", "trash"])]
        top: bool,
        #[arg(long, conflicts_with_all = ["item", "collection", "top"])]
        trash: bool,
    },
    Search {
        query: String,
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
        #[arg(long, default_value_t = 0)]
        start: u32,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long, value_enum)]
        direction: Option<SortDirection>,
        #[arg(long, value_enum, default_value_t = SearchMode::TitleCreatorYear)]
        qmode: SearchMode,
        #[arg(long)]
        include_trashed: bool,
    },
    Collections {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
        #[arg(long, default_value_t = 0)]
        start: u32,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long, value_enum)]
        direction: Option<SortDirection>,
        #[arg(long)]
        top: bool,
    },
    Groups {
        #[arg(long, default_value_t = 25, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
    },
    Item {
        key: String,
    },
    Children {
        key: String,
        #[arg(long, default_value_t = 25)]
        limit: u16,
    },
    Open {
        key: String,
        #[arg(long, help = "Open the Zotero web item page instead of the item's URL")]
        zotero: bool,
        #[arg(long, help = "Print the target instead of opening it")]
        print: bool,
    },
    Pdf {
        key: String,
        #[arg(short, long, help = "Write the PDF to this path or directory")]
        output: Option<PathBuf>,
        #[arg(long, help = "Print the downloaded path instead of opening it")]
        print: bool,
    },
    Attach {
        parent_key: String,
        path: PathBuf,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, default_value = "application/pdf")]
        content_type: String,
        #[arg(
            long,
            help = "Print the attachment item JSON without creating or uploading"
        )]
        dry_run: bool,
    },
    Update {
        key: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, help = "Replace the item's tags with this tag list")]
        tag: Vec<String>,
        #[arg(long, conflicts_with = "tag")]
        clear_tags: bool,
        #[arg(long, help = "Replace the item's collection membership with this list")]
        collection: Vec<String>,
        #[arg(long, conflicts_with = "collection")]
        clear_collections: bool,
        #[arg(long, help = "Print the PATCH JSON without updating")]
        dry_run: bool,
    },
    Delete {
        key: String,
        #[arg(long, help = "Required for deleting from Zotero")]
        yes: bool,
        #[arg(long, help = "Print the delete target and version without deleting")]
        dry_run: bool,
    },
    Export {
        format: ExportFormat,
        #[arg(long, conflicts_with_all = ["collection", "top", "trash"])]
        item: Option<String>,
        #[arg(long, conflicts_with = "trash")]
        collection: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u16,
        #[arg(long, conflicts_with_all = ["item", "trash"])]
        top: bool,
        #[arg(long, conflicts_with_all = ["item", "collection", "top"])]
        trash: bool,
        #[arg(short, long, help = "Write export output to this file")]
        output: Option<PathBuf>,
        #[arg(long, help = "CSL style for formatted bibliography exports")]
        style: Option<String>,
    },
    Add {
        #[command(subcommand)]
        command: AddCommands,
        #[arg(long, help = "Print the item JSON instead of creating it")]
        dry_run: bool,
        #[arg(long, help = "Collection key to add the created item to")]
        collection: Vec<String>,
        #[arg(long, help = "Tag to add to the created item")]
        tag: Vec<String>,
    },
    ResolveUser {
        username: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddCommands {
    Json {
        #[arg(long, help = "Inline Zotero item JSON object")]
        value: Option<String>,
        #[arg(
            default_value = "-",
            help = "Path to a Zotero item JSON object, inline JSON object, or - for stdin"
        )]
        input: String,
    },
    Doi {
        doi: String,
    },
    Isbn {
        isbn: String,
    },
    Url {
        url: String,
        #[arg(long)]
        title: Option<String>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum SearchMode {
    TitleCreatorYear,
    Everything,
}

impl SearchMode {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::TitleCreatorYear => "titleCreatorYear",
            Self::Everything => "everything",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExportFormat {
    Bibtex,
    Ris,
    Csljson,
    Bib,
}

impl ExportFormat {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Bibtex => "bibtex",
            Self::Ris => "ris",
            Self::Csljson => "csljson",
            Self::Bib => "bib",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum TagSearchMode {
    Contains,
    StartsWith,
}

impl TagSearchMode {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::StartsWith => "startsWith",
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn parses_inline_json_argument_for_add() {
        let cli = Cli::parse_from([
            "zot",
            "add",
            "--dry-run",
            "json",
            "{\"itemType\":\"webpage\"}",
        ]);

        match cli.command {
            Commands::Add {
                dry_run: true,
                collection,
                tag,
                command: AddCommands::Json { value: None, input },
            } => {
                assert!(collection.is_empty());
                assert!(tag.is_empty());
                assert_eq!(input, "{\"itemType\":\"webpage\"}");
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_explicit_value_for_add_json() {
        let cli = Cli::parse_from([
            "zot",
            "add",
            "--dry-run",
            "json",
            "--value",
            "{\"itemType\":\"webpage\"}",
            "item.json",
        ]);

        match cli.command {
            Commands::Add {
                dry_run: true,
                collection,
                tag,
                command: AddCommands::Json { value, input },
            } => {
                assert!(collection.is_empty());
                assert!(tag.is_empty());
                assert_eq!(value.as_deref(), Some("{\"itemType\":\"webpage\"}"));
                assert_eq!(input, "item.json");
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_items_collection_mode() {
        let cli = Cli::parse_from(["zot", "items", "--collection", "ABCD1234", "--top"]);

        match cli.command {
            Commands::Items {
                collection,
                limit,
                tag: None,
                top: true,
                trash: false,
                ..
            } => {
                assert_eq!(collection.as_deref(), Some("ABCD1234"));
                assert_eq!(limit, 25);
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_items_tag_filter() {
        let cli = Cli::parse_from(["zot", "items", "--tag", "neuroscience"]);

        match cli.command {
            Commands::Items {
                collection: None,
                limit: 25,
                tag,
                top: false,
                trash: false,
                ..
            } => {
                assert_eq!(tag.as_deref(), Some("neuroscience"));
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_items_pagination() {
        let cli = Cli::parse_from([
            "zot",
            "items",
            "--limit",
            "100",
            "--start",
            "25",
            "--all",
            "--sort",
            "title",
            "--direction",
            "asc",
        ]);

        match cli.command {
            Commands::Items {
                limit: 100,
                start: 25,
                all: true,
                sort,
                direction: Some(SortDirection::Asc),
                ..
            } => {
                assert_eq!(sort.as_deref(), Some("title"));
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn rejects_conflicting_items_modes() {
        assert!(Cli::try_parse_from(["zot", "items", "--trash", "--top"]).is_err());
        assert!(
            Cli::try_parse_from(["zot", "items", "--trash", "--collection", "ABCD1234"]).is_err()
        );
    }

    #[test]
    fn parses_export_item_mode() {
        let cli = Cli::parse_from(["zot", "export", "bibtex", "--item", "ABCD1234"]);

        match cli.command {
            Commands::Export {
                format: ExportFormat::Bibtex,
                item,
                collection: None,
                limit: 25,
                top: false,
                trash: false,
                output: None,
                style: None,
            } => {
                assert_eq!(item.as_deref(), Some("ABCD1234"));
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn rejects_conflicting_export_modes() {
        assert!(Cli::try_parse_from(["zot", "export", "bibtex", "--item", "A", "--top"]).is_err());
        assert!(
            Cli::try_parse_from(["zot", "export", "bibtex", "--trash", "--collection", "C"])
                .is_err()
        );
    }

    #[test]
    fn parses_tags_query() {
        let cli = Cli::parse_from(["zot", "tags", "--query", "neuro", "--qmode", "starts-with"]);

        match cli.command {
            Commands::Tags {
                item: None,
                collection: None,
                limit: 50,
                query,
                qmode: TagSearchMode::StartsWith,
                top: false,
                trash: false,
                ..
            } => {
                assert_eq!(query.as_deref(), Some("neuro"));
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn rejects_conflicting_tags_modes() {
        assert!(Cli::try_parse_from(["zot", "tags", "--item", "A", "--top"]).is_err());
        assert!(Cli::try_parse_from(["zot", "tags", "--trash", "--collection", "C"]).is_err());
    }

    #[test]
    fn parses_children_command() {
        let cli = Cli::parse_from(["zot", "children", "ABCD1234", "--limit", "5"]);

        match cli.command {
            Commands::Children { key, limit } => {
                assert_eq!(key, "ABCD1234");
                assert_eq!(limit, 5);
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_groups_command() {
        let cli = Cli::parse_from(["zot", "groups", "--limit", "5"]);

        match cli.command {
            Commands::Groups { limit } => assert_eq!(limit, 5),
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_attach_command() {
        let cli = Cli::parse_from([
            "zot",
            "attach",
            "PARENT1",
            "paper.pdf",
            "--title",
            "Paper",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Attach {
                parent_key,
                path,
                title,
                content_type,
                dry_run: true,
            } => {
                assert_eq!(parent_key, "PARENT1");
                assert_eq!(path, PathBuf::from("paper.pdf"));
                assert_eq!(title.as_deref(), Some("Paper"));
                assert_eq!(content_type, "application/pdf");
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_update_command() {
        let cli = Cli::parse_from([
            "zot",
            "update",
            "ITEM1",
            "--title",
            "New title",
            "--tag",
            "zot-live-test",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Update {
                key,
                title,
                tag,
                dry_run: true,
                ..
            } => {
                assert_eq!(key, "ITEM1");
                assert_eq!(title.as_deref(), Some("New title"));
                assert_eq!(tag, vec!["zot-live-test"]);
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_delete_command() {
        let cli = Cli::parse_from(["zot", "delete", "ITEM1", "--yes"]);

        match cli.command {
            Commands::Delete {
                key,
                yes: true,
                dry_run: false,
            } => assert_eq!(key, "ITEM1"),
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn parses_add_metadata_options() {
        let cli = Cli::parse_from([
            "zot",
            "add",
            "--dry-run",
            "--collection",
            "COLL1234",
            "--tag",
            "neuroscience",
            "json",
            "{\"itemType\":\"webpage\"}",
        ]);

        match cli.command {
            Commands::Add {
                dry_run: true,
                collection,
                tag,
                command: AddCommands::Json { .. },
            } => {
                assert_eq!(collection, vec!["COLL1234"]);
                assert_eq!(tag, vec!["neuroscience"]);
            }
            other => panic!("unexpected command shape: {other:?}"),
        }
    }
}
