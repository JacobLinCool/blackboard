mod auth;
mod board;
mod task;
mod user;

use std::cell::RefCell;
use std::io::{self, Write};

use self::task::{TaskAddInput, TaskEditInput};
use clap::Parser;
use rusqlite::Connection;
use serde_json::json;

use crate::cli::{BoardCmd, Cli, Cmd, TaskCmd, UserCmd};
use crate::db::{db_path, init_db, open_db};
use crate::error::{AppErr, Res};
use crate::utils::parse_depends;

pub struct App {
    conn: Connection,
    json_output: bool,
    json_lines: RefCell<Vec<String>>,
    json_streaming: RefCell<bool>,
}

impl App {
    pub fn run() -> Res<()> {
        let cli = Cli::parse();
        match cli.cmd {
            Cmd::Clear => Self::run_clear(cli.json),
            cmd => {
                let mut app = Self::new(cli.json)?;
                app.dispatch(cmd)?;
                if app.json_output && !*app.json_streaming.borrow() {
                    let payload = json!({
                        "ok": true,
                        "lines": app.json_lines.into_inner(),
                    });
                    println!("{}", payload);
                }
                Ok(())
            }
        }
    }

    fn new(json_output: bool) -> Res<Self> {
        let conn = open_db()?;
        init_db(&conn)?;
        Ok(Self {
            conn,
            json_output,
            json_lines: RefCell::new(Vec::new()),
            json_streaming: RefCell::new(false),
        })
    }

    fn dispatch(&mut self, cmd: Cmd) -> Res<()> {
        match cmd {
            Cmd::Init { user } => self.cmd_init(&user),
            Cmd::Clear => unreachable!("clear is handled before opening the database"),
            Cmd::Board(cmd) => match cmd {
                BoardCmd::Create { user, name } => self.cmd_board_create(&user, &name),
                BoardCmd::List { user } => self.cmd_board_list(&user),
                BoardCmd::View { user, board } => self.cmd_board_view(&user, &board),
                BoardCmd::Members { user, board } => self.cmd_board_members(&user, &board),
                BoardCmd::Poll {
                    user,
                    board,
                    interval,
                    idle_notice_secs,
                } => self.cmd_board_poll(&user, &board, interval, idle_notice_secs),
                BoardCmd::Grant {
                    user,
                    board,
                    target,
                    permissions,
                } => {
                    let mut actions: Vec<String> = permissions
                        .into_iter()
                        .map(|permission| permission.as_str().to_string())
                        .collect();
                    if actions.is_empty() {
                        actions.push("read".to_string());
                    }
                    self.cmd_board_grant(&user, &board, &target, actions)
                }
                BoardCmd::Revoke {
                    user,
                    board,
                    target,
                } => self.cmd_board_revoke(&user, &board, &target),
                BoardCmd::Delete { user, board } => self.cmd_board_delete(&user, &board),
            },
            Cmd::Task(cmd) => match cmd {
                TaskCmd::List {
                    user,
                    board,
                    status,
                    size,
                    parent,
                    assignee,
                } => self.cmd_task_list(
                    &user,
                    &board,
                    status.as_ref().map(|value| value.as_str()),
                    size.as_ref().map(|value| value.as_str()),
                    parent,
                    assignee.as_deref(),
                ),
                TaskCmd::View {
                    user,
                    board,
                    task_id,
                } => self.cmd_task_view(&user, &board, task_id),
                TaskCmd::Add {
                    user,
                    board,
                    title,
                    description,
                    size,
                    parent,
                    assignee,
                    depends_on,
                } => self.cmd_task_add(TaskAddInput {
                    actor: user,
                    board,
                    title,
                    description,
                    size: size.as_str().to_string(),
                    parent,
                    assignee,
                    depends: parse_depends(depends_on)?,
                }),
                TaskCmd::Edit {
                    user,
                    board,
                    task_id,
                    title,
                    description,
                    size,
                    parent,
                    assignee,
                    depends_on,
                    clear_depends_on,
                } => {
                    let deps = if clear_depends_on {
                        Some(Vec::new())
                    } else {
                        parse_depends(depends_on)?
                    };
                    self.cmd_task_edit(TaskEditInput {
                        actor: user,
                        board,
                        task_id,
                        title,
                        description,
                        size: size.map(|value| value.as_str().to_string()),
                        parent,
                        assignee,
                        depends: deps,
                    })
                }
                TaskCmd::Status {
                    user,
                    board,
                    task_id,
                    status,
                    note,
                } => self.cmd_task_status(&user, &board, task_id, status.as_str(), note),
                TaskCmd::Delete {
                    user,
                    board,
                    task_id,
                } => self.cmd_task_delete(&user, &board, task_id),
            },
            Cmd::User(cmd) => match cmd {
                UserCmd::Add { user, name } => self.cmd_user_add(&user, &name),
                UserCmd::Remove { user, name } => self.cmd_user_remove(&user, &name),
                UserCmd::List { user } => self.cmd_user_list(&user),
            },
        }
    }

    fn run_clear(json_output: bool) -> Res<()> {
        Self::require_system_root()?;

        let db_path = db_path()?;
        let line = if db_path.exists() {
            std::fs::remove_file(&db_path)?;
            format!("cleared all data by removing {}", db_path.display())
        } else {
            format!("database already absent at {}", db_path.display())
        };

        if json_output {
            let payload = json!({
                "ok": true,
                "lines": [line],
            });
            println!("{}", payload);
        } else {
            println!("{}", line);
        }

        Ok(())
    }

    fn require_system_root() -> Res<()> {
        #[cfg(unix)]
        {
            let euid = unsafe {
                // SAFETY: libc::geteuid has no preconditions and does not dereference pointers.
                libc::geteuid()
            };
            if euid == 0 {
                return Ok(());
            }
            Err(Box::new(AppErr(
                "forbidden",
                "clear requires system root privileges (effective uid 0)".into(),
            )))
        }

        #[cfg(not(unix))]
        {
            Err(Box::new(AppErr(
                "system",
                "clear is only supported on unix platforms".into(),
            )))
        }
    }

    fn emit_line(&self, line: impl Into<String>) {
        let line = line.into();
        if self.json_output {
            self.json_lines.borrow_mut().push(line);
        } else {
            println!("{}", line);
        }
    }

    fn enable_json_streaming(&self) {
        self.json_streaming.replace(true);
    }

    fn emit_realtime_line(&self, line: impl Into<String>) {
        let line = line.into();
        if self.json_output {
            let payload = json!({
                "ok": true,
                "line": line,
            });
            println!("{}", payload);
        } else {
            println!("{}", line);
        }
        let _ = io::stdout().flush();
    }
}
