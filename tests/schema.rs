mod common;

use common::TestEnv;
use rusqlite::Connection;
use std::fs;

#[test]
fn schema_matches_single_current_model() {
    let env = TestEnv::new();
    env.run_ok(&["init", "--user", "alice"]);

    let conn = Connection::open(env.db_path()).expect("failed to open db");

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='board_member_permissions'",
            [],
            |row| row.get(0),
        )
        .expect("failed to inspect sqlite_master");
    assert_eq!(table_count, 1);

    let board_member_permissions_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='board_member_permissions'",
            [],
            |row| row.get(0),
        )
        .expect("failed to inspect board_member_permissions table definition");
    assert!(board_member_permissions_sql.contains("delete_board"));

    let mut statement = conn
        .prepare("PRAGMA table_info(tasks)")
        .expect("failed to query tasks schema");
    let mut rows = statement.query([]).expect("failed to query rows");

    let mut columns = Vec::new();
    while let Some(row) = rows.next().expect("failed to read row") {
        columns.push(row.get::<_, String>(1).expect("failed to read column name"));
    }

    assert!(columns.contains(&"status".to_string()));
    assert!(columns.contains(&"assignee_id".to_string()));
    assert!(!columns.contains(&"kind".to_string()));

    let mut members_statement = conn
        .prepare("PRAGMA table_info(board_members)")
        .expect("failed to query board_members schema");
    let mut member_rows = members_statement
        .query([])
        .expect("failed to query board_members rows");

    let mut member_columns = Vec::new();
    while let Some(row) = member_rows.next().expect("failed to read member row") {
        member_columns.push(
            row.get::<_, String>(1)
                .expect("failed to read member column name"),
        );
    }
    assert!(member_columns.contains(&"board_id".to_string()));
    assert!(member_columns.contains(&"user_id".to_string()));
    assert!(!member_columns.contains(&"role_id".to_string()));

    let legacy_roles_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='roles'",
            [],
            |row| row.get(0),
        )
        .expect("failed to inspect sqlite_master for roles");
    assert_eq!(legacy_roles_count, 0);

    let legacy_permissions_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='permissions'",
            [],
            |row| row.get(0),
        )
        .expect("failed to inspect sqlite_master for permissions");
    assert_eq!(legacy_permissions_count, 0);
}

#[test]
fn legacy_schema_is_rejected() {
    let env = TestEnv::new();
    let db_dir = env.home_path().join(".blackboard");
    fs::create_dir_all(&db_dir).expect("failed to create legacy db dir");
    let db_path = db_dir.join("blackboard.db");

    let conn = Connection::open(&db_path).expect("failed to open legacy db");
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
CREATE TABLE users(id INTEGER PRIMARY KEY,name TEXT UNIQUE NOT NULL);
CREATE TABLE boards(id INTEGER PRIMARY KEY,name TEXT UNIQUE NOT NULL,owner_id INTEGER NOT NULL,created_at INTEGER NOT NULL,FOREIGN KEY(owner_id) REFERENCES users(id) ON DELETE RESTRICT);
CREATE TABLE board_members(board_id INTEGER NOT NULL,user_id INTEGER NOT NULL,role_id INTEGER NOT NULL,PRIMARY KEY(board_id,user_id,role_id),FOREIGN KEY(board_id) REFERENCES boards(id) ON DELETE CASCADE,FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE);
CREATE TABLE tasks(id INTEGER PRIMARY KEY,board_id INTEGER NOT NULL,title TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',parent_id INTEGER,kind TEXT NOT NULL CHECK(kind IN ('main','project','implementation')),assignee_id INTEGER,status TEXT NOT NULL CHECK(status IN ('pending','in_progress','completed','blocked')),created_by INTEGER,updated_by INTEGER,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,FOREIGN KEY(board_id) REFERENCES boards(id) ON DELETE CASCADE,FOREIGN KEY(parent_id) REFERENCES tasks(id) ON DELETE CASCADE);
CREATE TABLE task_dependencies(task_id INTEGER NOT NULL,depends_on_task_id INTEGER NOT NULL,PRIMARY KEY(task_id,depends_on_task_id),CHECK(task_id != depends_on_task_id),FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,FOREIGN KEY(depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE);",
    )
    .expect("failed to seed legacy schema");

    let err = env.run_err(&["init", "--user", "alice"]);
    assert!(err.contains("schema:"));
}
