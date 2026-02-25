use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::error::{AppErr, Res};
use crate::utils::now_secs;

use super::App;

impl App {
    pub(super) fn cmd_board_create(&mut self, actor: &str, name: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        tx.execute(
            "INSERT INTO boards(name,owner_id,created_at) VALUES (?1,?2,?3)",
            params![name, actor_id, now_secs()],
        )?;
        let board_id = tx.last_insert_rowid();
        tx.commit()?;

        self.emit_line(format!("created board {} (id={})", name, board_id));
        Ok(())
    }

    pub(super) fn cmd_board_list(&self, actor: &str) -> Res<()> {
        let actor_id = self.get_user_id(actor)?;
        let mut statement = self.conn.prepare(
            "SELECT DISTINCT b.id,b.name,u.name,b.created_at FROM boards b JOIN users u ON u.id=b.owner_id LEFT JOIN board_member_permissions bmp ON bmp.board_id=b.id AND bmp.user_id=?1 AND bmp.action='read' WHERE b.owner_id=?1 OR bmp.user_id IS NOT NULL ORDER BY b.id",
        )?;
        let mut rows = statement.query([actor_id])?;

        self.emit_line(format!(
            "{:<4} {:<18} {:<14} CREATED",
            "ID", "NAME", "OWNER"
        ));
        while let Some(row) = rows.next()? {
            self.emit_line(format!(
                "{:<4} {:<18} {:<14} {}",
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?
            ));
        }

        Ok(())
    }

    pub(super) fn cmd_board_view(&self, actor: &str, board: &str) -> Res<()> {
        let actor_id = self.get_user_id(actor)?;
        let board_id = self.resolve_board_id(board)?;
        self.require_action(actor_id, board_id, "read")?;

        let (id, name, owner_id, created): (i64, String, i64, i64) = self.conn.query_row(
            "SELECT id,name,owner_id,created_at FROM boards WHERE id=?1",
            [board_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        let owner: String =
            self.conn
                .query_row("SELECT name FROM users WHERE id=?1", [owner_id], |row| {
                    row.get(0)
                })?;
        let total_tasks: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE board_id=?1",
            [board_id],
            |row| row.get(0),
        )?;

        self.emit_line(format!(
            "board {} (id={}) owner={} created={}",
            name, id, owner, created
        ));
        self.emit_line(format!("tasks total={}", total_tasks));

        Ok(())
    }

    pub(super) fn cmd_board_members(&self, actor: &str, board: &str) -> Res<()> {
        let actor_id = self.get_user_id(actor)?;
        let board_id = self.resolve_board_id(board)?;
        self.require_action(actor_id, board_id, "read")?;

        let (owner_id, owner_name): (i64, String) = self.conn.query_row(
            "SELECT u.id,u.name FROM boards b JOIN users u ON u.id=b.owner_id WHERE b.id=?1",
            [board_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        self.emit_line("members:".to_string());
        self.emit_line(format!("  {} [owner] perms=all", owner_name));

        let mut statement = self.conn.prepare(
            "SELECT u.id,u.name FROM board_members bm JOIN users u ON u.id=bm.user_id WHERE bm.board_id=?1 AND bm.user_id!=?2 ORDER BY u.name",
        )?;
        let mut rows = statement.query(params![board_id, owner_id])?;
        while let Some(row) = rows.next()? {
            let member_id = row.get::<_, i64>(0)?;
            let member_name = row.get::<_, String>(1)?;
            let permissions = self.member_permissions(board_id, member_id)?;
            self.emit_line(format!(
                "  {} [member] perms={}",
                member_name,
                permissions.join(",")
            ));
        }

        Ok(())
    }

    pub(super) fn cmd_board_poll(
        &mut self,
        actor: &str,
        board: &str,
        interval_secs: u64,
        idle_notice_secs: u64,
    ) -> Res<()> {
        if interval_secs == 0 {
            return Err(Box::new(AppErr("input", "--interval must be >= 1".into())));
        }
        if idle_notice_secs == 0 {
            return Err(Box::new(AppErr(
                "input",
                "--idle-notice-secs must be >= 1".into(),
            )));
        }

        let actor_id = self.get_user_id(actor)?;
        let board_id = self.resolve_board_id(board)?;
        self.require_action(actor_id, board_id, "read")?;

        self.enable_json_streaming();
        self.emit_realtime_line(format!(
            "polling board {} (id={}) every {}s (idle notice {}s)",
            board, board_id, interval_secs, idle_notice_secs
        ));

        let init_tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)?;
        Self::require_action_on(&init_tx, actor_id, board_id, "read")?;
        let mut previous = Self::capture_board_snapshot(&init_tx, board_id)?;
        init_tx.commit()?;
        let mut last_idle_notice = Instant::now();

        loop {
            thread::sleep(Duration::from_secs(interval_secs));

            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            Self::require_action_on(&tx, actor_id, board_id, "read")?;
            let current = Self::capture_board_snapshot(&tx, board_id)?;
            tx.commit()?;

            if current != previous {
                self.emit_realtime_line(format!(
                    "update detected: tasks={} members={} permissions={}",
                    current.task_count, current.member_count, current.permission_count
                ));
                previous = current;
                last_idle_notice = Instant::now();
            } else if last_idle_notice.elapsed() >= Duration::from_secs(idle_notice_secs) {
                self.emit_realtime_line(format!("no update in last {}s", idle_notice_secs));
                last_idle_notice = Instant::now();
            }
        }
    }

    pub(super) fn cmd_board_grant(
        &mut self,
        actor: &str,
        board: &str,
        target: &str,
        permissions: Vec<String>,
    ) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "assign")?;

        let target_id = Self::get_or_create_user_on(&tx, target)?;
        if Self::is_board_owner_on(&tx, target_id, board_id)? {
            tx.commit()?;
            self.emit_line("already owner".to_string());
            return Ok(());
        }

        let changed_rows = tx.execute(
            "INSERT OR IGNORE INTO board_members(board_id,user_id) VALUES (?1,?2)",
            params![board_id, target_id],
        )?;
        let normalized = Self::set_member_permissions_on(&tx, board_id, target_id, &permissions)?;
        tx.commit()?;
        self.emit_line(
            if changed_rows == 0 {
                "updated permissions"
            } else {
                "granted"
            }
            .to_string(),
        );
        self.emit_line(format!("permissions: {}", normalized.join(",")));

        Ok(())
    }

    pub(super) fn cmd_board_revoke(&mut self, actor: &str, board: &str, target: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "assign")?;

        let target_id = Self::get_user_id_on(&tx, target)?;
        if Self::is_board_owner_on(&tx, target_id, board_id)? {
            return Err(Box::new(AppErr(
                "forbidden",
                "cannot revoke board owner".into(),
            )));
        }

        let changed_rows = tx.execute(
            "DELETE FROM board_members WHERE board_id=?1 AND user_id=?2",
            params![board_id, target_id],
        )?;
        tx.commit()?;
        self.emit_line(format!("revoked {} row(s)", changed_rows));

        Ok(())
    }

    pub(super) fn cmd_board_delete(&mut self, actor: &str, board: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "delete_board")?;

        let changed_rows = tx.execute("DELETE FROM boards WHERE id=?1", params![board_id])?;
        tx.commit()?;
        self.emit_line(format!("deleted {} row(s)", changed_rows));

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BoardSnapshot {
    task_count: i64,
    member_count: i64,
    permission_count: i64,
    fingerprint: String,
}

impl App {
    fn capture_board_snapshot(conn: &Connection, board_id: i64) -> Res<BoardSnapshot> {
        let owner_id: i64 = conn
            .query_row(
                "SELECT owner_id FROM boards WHERE id=?1",
                [board_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppErr("not_found", "board not found".into()))?;

        let task_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE board_id=?1",
            [board_id],
            |row| row.get(0),
        )?;
        let member_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM board_members WHERE board_id=?1",
            [board_id],
            |row| row.get(0),
        )?;
        let permission_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM board_member_permissions WHERE board_id=?1",
            [board_id],
            |row| row.get(0),
        )?;

        let mut fingerprint = format!("owner:{};", owner_id);

        let mut task_stmt = conn.prepare(
            "SELECT id,updated_at,status,parent_id,assignee_id,title,description,size FROM tasks WHERE board_id=?1 ORDER BY id",
        )?;
        let mut task_rows = task_stmt.query([board_id])?;
        while let Some(row) = task_rows.next()? {
            fingerprint.push_str(&format!(
                "t:{}:{}:{}:{:?}:{:?}:{}:{}:{};",
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ));
        }

        let mut dep_stmt = conn.prepare(
            "SELECT td.task_id,td.depends_on_task_id FROM task_dependencies td JOIN tasks t ON t.id=td.task_id WHERE t.board_id=?1 ORDER BY td.task_id,td.depends_on_task_id",
        )?;
        let mut dep_rows = dep_stmt.query([board_id])?;
        while let Some(row) = dep_rows.next()? {
            fingerprint.push_str(&format!(
                "d:{}:{};",
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
            ));
        }

        let mut member_stmt = conn.prepare(
            "SELECT bm.user_id,p.action FROM board_members bm LEFT JOIN board_member_permissions p ON p.board_id=bm.board_id AND p.user_id=bm.user_id WHERE bm.board_id=?1 ORDER BY bm.user_id,p.action",
        )?;
        let mut member_rows = member_stmt.query([board_id])?;
        while let Some(row) = member_rows.next()? {
            fingerprint.push_str(&format!(
                "m:{}:{};",
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?
                    .unwrap_or_else(|| "-".to_string())
            ));
        }

        Ok(BoardSnapshot {
            task_count,
            member_count,
            permission_count,
            fingerprint,
        })
    }
}
