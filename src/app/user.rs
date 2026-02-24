use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::error::{AppErr, Res};

use super::App;

impl App {
    pub(super) fn cmd_init(&mut self, actor: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let user_id = Self::get_or_create_user_on(&tx, actor)?;
        tx.commit()?;
        self.emit_line(format!("initialized user {} ({})", actor, user_id));
        Ok(())
    }

    pub(super) fn cmd_user_add(&mut self, actor: &str, name: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let _ = Self::get_user_id_on(&tx, actor)?;
        let user_id = Self::get_or_create_user_on(&tx, name)?;
        tx.commit()?;
        self.emit_line(format!("added user {} ({})", name, user_id));
        Ok(())
    }

    pub(super) fn cmd_user_remove(&mut self, actor: &str, name: &str) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let _ = Self::get_user_id_on(&tx, actor)?;
        let changed_rows = tx.execute("DELETE FROM users WHERE name=?1", params![name])?;
        tx.commit()?;
        self.emit_line(format!("removed {} row(s)", changed_rows));
        Ok(())
    }

    pub(super) fn cmd_user_list(&self, actor: &str) -> Res<()> {
        let _ = self.get_user_id(actor)?;
        let mut statement = self.conn.prepare("SELECT id,name FROM users ORDER BY id")?;
        let mut rows = statement.query([])?;

        self.emit_line(format!("{:<4} NAME", "ID"));
        while let Some(row) = rows.next()? {
            self.emit_line(format!(
                "{:<4} {}",
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?
            ));
        }
        Ok(())
    }

    pub(super) fn get_user_id(&self, name: &str) -> Res<i64> {
        Self::get_user_id_on(&self.conn, name)
    }

    pub(super) fn get_user_id_on(conn: &Connection, name: &str) -> Res<i64> {
        conn.query_row("SELECT id FROM users WHERE name=?1", [name], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .ok_or_else(|| AppErr("auth", format!("user '{}' not found", name)).into())
    }

    pub(super) fn get_or_create_user_on(conn: &Connection, name: &str) -> Res<i64> {
        conn.execute("INSERT OR IGNORE INTO users(name) VALUES (?1)", [name])?;
        conn.query_row("SELECT id FROM users WHERE name=?1", [name], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .ok_or_else(|| {
            AppErr(
                "system",
                format!("failed to resolve user id after upsert for '{}'", name),
            )
            .into()
        })
    }
}
