use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AppErr, Res};

use super::App;

const ACTIONS: [&str; 7] = [
    "read",
    "create",
    "update",
    "delete",
    "set_status",
    "assign",
    "delete_board",
];

impl App {
    pub(super) fn resolve_board_id(&self, selector: &str) -> Res<i64> {
        Self::resolve_board_id_on(&self.conn, selector)
    }

    pub(super) fn require_action(&self, actor_id: i64, board_id: i64, action: &str) -> Res<()> {
        Self::require_action_on(&self.conn, actor_id, board_id, action)
    }

    pub(super) fn member_permissions(&self, board_id: i64, user_id: i64) -> Res<Vec<String>> {
        Self::member_permissions_on(&self.conn, board_id, user_id)
    }

    pub(super) fn resolve_board_id_on(conn: &Connection, selector: &str) -> Res<i64> {
        if let Some(id) = conn
            .query_row("SELECT id FROM boards WHERE name=?1", [selector], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?
        {
            return Ok(id);
        }

        let id = selector
            .parse::<i64>()
            .map_err(|_| AppErr("input", "board selector must be board name or id".into()))?;
        conn.query_row("SELECT id FROM boards WHERE id=?1", [id], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .ok_or_else(|| AppErr("not_found", "board not found".into()).into())
    }

    pub(super) fn require_action_on(
        conn: &Connection,
        actor_id: i64,
        board_id: i64,
        action: &str,
    ) -> Res<()> {
        if Self::is_board_owner_on(conn, actor_id, board_id)? {
            return Ok(());
        }

        if !ACTIONS.contains(&action) {
            return Err(Box::new(AppErr(
                "system",
                format!("unknown action '{}'", action),
            )));
        }

        if Self::has_member_permission_on(conn, actor_id, board_id, action)? {
            return Ok(());
        }
        Err(Box::new(AppErr("forbidden", format!("{} denied", action))))
    }

    pub(super) fn member_permissions_on(
        conn: &Connection,
        board_id: i64,
        user_id: i64,
    ) -> Res<Vec<String>> {
        let mut statement = conn.prepare(
            "SELECT action FROM board_member_permissions WHERE board_id=?1 AND user_id=?2",
        )?;
        let mut rows = statement.query(params![board_id, user_id])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get::<_, String>(0)?);
        }
        Ok(Self::normalize_action_list(out))
    }

    pub(super) fn set_member_permissions_on(
        conn: &Connection,
        board_id: i64,
        user_id: i64,
        actions: &[String],
    ) -> Res<Vec<String>> {
        let normalized = Self::normalize_action_list(actions.to_vec());
        if normalized.is_empty() {
            return Err(Box::new(AppErr(
                "input",
                "at least one permission is required".into(),
            )));
        }
        for action in &normalized {
            if !ACTIONS.contains(&action.as_str()) {
                return Err(Box::new(AppErr(
                    "input",
                    format!("unknown permission '{}'", action),
                )));
            }
        }

        conn.execute(
            "DELETE FROM board_member_permissions WHERE board_id=?1 AND user_id=?2",
            params![board_id, user_id],
        )?;
        for action in &normalized {
            conn.execute(
                "INSERT INTO board_member_permissions(board_id,user_id,action) VALUES (?1,?2,?3)",
                params![board_id, user_id, action],
            )?;
        }
        Ok(normalized)
    }

    pub(super) fn is_board_owner_on(conn: &Connection, actor_id: i64, board_id: i64) -> Res<bool> {
        let owner_id: Option<i64> = conn
            .query_row(
                "SELECT owner_id FROM boards WHERE id=?1",
                [board_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(owner_id == Some(actor_id))
    }

    fn has_member_permission_on(
        conn: &Connection,
        actor_id: i64,
        board_id: i64,
        action: &str,
    ) -> Res<bool> {
        let found: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM board_member_permissions WHERE board_id=?1 AND user_id=?2 AND action=?3 LIMIT 1",
                params![board_id, actor_id, action],
                |row| row.get(0),
            )
            .optional()?;
        Ok(found.is_some())
    }

    fn normalize_action_list(mut actions: Vec<String>) -> Vec<String> {
        actions.sort_unstable();
        actions.dedup();
        ACTIONS
            .iter()
            .filter(|name| actions.iter().any(|action| action == *name))
            .map(|name| (*name).to_string())
            .collect()
    }
}
