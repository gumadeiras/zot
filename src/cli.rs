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
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u16,
        #[arg(long, value_enum, default_value_t = SearchMode::TitleCreatorYear)]
        qmode: SearchMode,
        #[arg(long)]
        include_trashed: bool,
    },
    Collections {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u16,
        #[arg(long)]
        top: bool,
    },
    Item {
        key: String,
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
    Add {
        #[command(subcommand)]
        command: AddCommands,
        #[arg(long, help = "Print the item JSON instead of creating it")]
        dry_run: bool,
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
