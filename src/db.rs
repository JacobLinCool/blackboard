use std::path::PathBuf;
use std::time::Duration;

use dirs::home_dir;
use rusqlite::Connection;

use crate::error::{AppErr, Res};

pub fn open_db() -> Res<Connection> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

pub fn db_path() -> Res<PathBuf> {
    let mut path = home_dir().ok_or_else(|| AppErr("env", "cannot resolve home dir".into()))?;
    path.push(".blackboard");
    path.push("blackboard.db");
    Ok(path)
}

pub fn init_db(conn: &Connection) -> Res<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users(
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS boards(
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    owner_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(owner_id) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS board_members(
    board_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    PRIMARY KEY(board_id, user_id),
    FOREIGN KEY(board_id) REFERENCES boards(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS board_member_permissions(
    board_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('read','create','update','delete','set_status','assign','delete_board')),
    PRIMARY KEY(board_id, user_id, action),
    FOREIGN KEY(board_id, user_id) REFERENCES board_members(board_id, user_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tasks(
    id INTEGER PRIMARY KEY,
    board_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    parent_id INTEGER,
    assignee_id INTEGER,
    status TEXT NOT NULL CHECK(status IN ('pending','in_progress','completed','blocked')),
    created_by INTEGER,
    updated_by INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(board_id) REFERENCES boards(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_dependencies(
    task_id INTEGER NOT NULL,
    depends_on_task_id INTEGER NOT NULL,
    PRIMARY KEY(task_id, depends_on_task_id),
    CHECK(task_id != depends_on_task_id),
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY(depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);",
    )?;

    validate_schema(conn)
}

fn validate_schema(conn: &Connection) -> Res<()> {
    ensure_table_exists(conn, "board_member_permissions")?;
    ensure_board_permission_schema(conn)?;

    let task_columns = table_columns(conn, "tasks")?;
    if task_columns.iter().any(|col| col == "kind") {
        return Err(Box::new(AppErr(
            "schema",
            "unsupported legacy schema detected: tasks.kind still exists; remove ~/.blackboard/blackboard.db".into(),
        )));
    }

    let member_columns = table_columns(conn, "board_members")?;
    if member_columns.iter().any(|col| col == "role_id") {
        return Err(Box::new(AppErr(
            "schema",
            "unsupported legacy schema detected: board_members.role_id still exists; remove ~/.blackboard/blackboard.db".into(),
        )));
    }

    Ok(())
}

fn ensure_board_permission_schema(conn: &Connection) -> Res<()> {
    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='board_member_permissions'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| {
            AppErr(
                "schema",
                "table 'board_member_permissions' is missing".into(),
            )
        })?;

    if !table_sql.contains("delete_board") {
        return Err(Box::new(AppErr(
            "schema",
            "unsupported schema detected: board_member_permissions is missing delete_board permission; remove ~/.blackboard/blackboard.db".into(),
        )));
    }

    Ok(())
}

fn ensure_table_exists(conn: &Connection, table: &str) -> Res<()> {
    let exists: Option<i64> = conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        [table],
        |row| row.get(0),
    )?;

    if exists.is_none() {
        return Err(Box::new(AppErr(
            "schema",
            format!("table '{}' is missing", table),
        )));
    }

    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Res<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let mut rows = statement.query([])?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next()? {
        columns.push(row.get::<_, String>(1)?);
    }
    Ok(columns)
}
