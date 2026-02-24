use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "blackboard", version, about = "Multi-board taskboard CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    Init {
        #[arg(long)]
        user: String,
    },
    Clear,
    #[command(subcommand)]
    Board(BoardCmd),
    #[command(subcommand)]
    Task(TaskCmd),
    #[command(subcommand)]
    User(UserCmd),
}

#[derive(Subcommand)]
pub enum BoardCmd {
    Create {
        #[arg(long)]
        user: String,
        #[arg(long)]
        name: String,
    },
    List {
        #[arg(long)]
        user: String,
    },
    View {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
    },
    Members {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
    },
    Poll {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long, default_value_t = 1)]
        interval: u64,
        #[arg(long, default_value_t = 30)]
        idle_notice_secs: u64,
    },
    Grant {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        target: String,
        #[arg(long, value_delimiter = ',')]
        permissions: Vec<PermissionArg>,
    },
    Revoke {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        target: String,
    },
    Delete {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
    },
}

#[derive(Subcommand)]
pub enum TaskCmd {
    List {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        status: Option<StatusArg>,
        #[arg(long)]
        parent: Option<i64>,
        #[arg(long)]
        assignee: Option<String>,
    },
    View {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        task_id: i64,
    },
    Add {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        parent: Option<i64>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        depends_on: Option<String>,
    },
    Edit {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        task_id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        parent: Option<i64>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        depends_on: Option<String>,
        #[arg(long)]
        clear_depends_on: bool,
    },
    Status {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        task_id: i64,
        #[arg(long)]
        status: StatusArg,
    },
    Delete {
        #[arg(long)]
        user: String,
        #[arg(long)]
        board: String,
        #[arg(long)]
        task_id: i64,
    },
}

#[derive(Subcommand)]
pub enum UserCmd {
    Add {
        #[arg(long)]
        user: String,
        #[arg(long)]
        name: String,
    },
    Remove {
        #[arg(long)]
        user: String,
        #[arg(long)]
        name: String,
    },
    List {
        #[arg(long)]
        user: String,
    },
}

#[derive(ValueEnum, Clone)]
pub enum StatusArg {
    Pending,
    #[value(name = "in_progress")]
    InProgress,
    Completed,
    Blocked,
}

impl StatusArg {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusArg::Pending => "pending",
            StatusArg::InProgress => "in_progress",
            StatusArg::Completed => "completed",
            StatusArg::Blocked => "blocked",
        }
    }
}

#[derive(ValueEnum, Clone)]
pub enum PermissionArg {
    Read,
    Create,
    Update,
    Delete,
    #[value(name = "set_status")]
    SetStatus,
    Assign,
    #[value(name = "delete_board")]
    DeleteBoard,
}

impl PermissionArg {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionArg::Read => "read",
            PermissionArg::Create => "create",
            PermissionArg::Update => "update",
            PermissionArg::Delete => "delete",
            PermissionArg::SetStatus => "set_status",
            PermissionArg::Assign => "assign",
            PermissionArg::DeleteBoard => "delete_board",
        }
    }
}
