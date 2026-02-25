use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

use crate::error::{AppErr, Res};
use crate::models::Task;
use crate::utils::{format_depends, normalize_depends, now_secs};

use super::App;

pub(super) struct TaskAddInput {
    pub(super) actor: String,
    pub(super) board: String,
    pub(super) title: String,
    pub(super) description: String,
    pub(super) size: String,
    pub(super) parent: Option<i64>,
    pub(super) assignee: Option<String>,
    pub(super) depends: Option<Vec<i64>>,
}

pub(super) struct TaskEditInput {
    pub(super) actor: String,
    pub(super) board: String,
    pub(super) task_id: i64,
    pub(super) title: Option<String>,
    pub(super) description: Option<String>,
    pub(super) size: Option<String>,
    pub(super) parent: Option<i64>,
    pub(super) assignee: Option<String>,
    pub(super) depends: Option<Vec<i64>>,
}

struct TaskNote {
    created_at: i64,
    status: String,
    author_name: Option<String>,
    body: String,
}

impl App {
    pub(super) fn cmd_task_list(
        &self,
        actor: &str,
        board: &str,
        status: Option<&str>,
        size: Option<&str>,
        parent: Option<i64>,
        assignee: Option<&str>,
    ) -> Res<()> {
        let actor_id = self.get_user_id(actor)?;
        let board_id = self.resolve_board_id(board)?;
        self.require_action(actor_id, board_id, "read")?;

        let assignee_id = match assignee {
            Some(name) => Some(self.get_user_id(name)?),
            None => None,
        };
        let mut statement = self.conn.prepare("SELECT t.id,t.title,t.description,t.size,t.parent_id,t.assignee_id,u.name,t.status FROM tasks t LEFT JOIN users u ON u.id=t.assignee_id WHERE t.board_id=?1 ORDER BY t.id")?;
        let mut rows = statement.query([board_id])?;

        self.emit_line(format!(
            "{:<4} {:<12} {:<8} {:<7} {:<12} {:<12} TITLE",
            "ID", "STATUS", "SIZE", "PARENT", "ASSIGNEE", "dependsOn"
        ));
        while let Some(row) = rows.next()? {
            let task = Self::map_task(row)?;
            if let Some(value) = status
                && task.status != value
            {
                continue;
            }
            if let Some(value) = size
                && task.size != value
            {
                continue;
            }
            if let Some(value) = parent
                && task.parent_id != Some(value)
            {
                continue;
            }
            if let Some(value) = assignee_id
                && task.assignee_id != Some(value)
            {
                continue;
            }

            self.emit_line(format!(
                "{:<4} {:<12} {:<8} {:<7} {:<12} {:<12} {}",
                task.id,
                task.status,
                task.size,
                task.parent_id
                    .map_or("-".to_string(), |value| value.to_string()),
                task.assignee_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                format_depends(&Self::depends_of(&self.conn, task.id)?),
                task.title
            ));
        }

        Ok(())
    }

    pub(super) fn cmd_task_view(&self, actor: &str, board: &str, task_id: i64) -> Res<()> {
        let actor_id = self.get_user_id(actor)?;
        let board_id = self.resolve_board_id(board)?;
        self.require_action(actor_id, board_id, "read")?;

        let task = self.get_task(board_id, task_id)?;
        self.emit_line(format!("id: {}", task.id));
        self.emit_line(format!("title: {}", task.title));
        self.emit_line(format!("description: {}", task.description));
        self.emit_line(format!("size: {}", task.size));
        self.emit_line(format!("status: {}", task.status));
        self.emit_line(format!("parent: {:?}", task.parent_id));
        self.emit_line(format!(
            "assignee: {}",
            task.assignee_name.unwrap_or_else(|| "-".to_string())
        ));
        self.emit_line(format!(
            "dependsOn: {}",
            format_depends(&Self::depends_of(&self.conn, task_id)?)
        ));

        let notes = Self::notes_of(&self.conn, task_id)?;
        self.emit_line("postNotes:".to_string());
        if notes.is_empty() {
            self.emit_line("  - (none)".to_string());
        }
        for note in notes {
            self.emit_line(format!(
                "  - [{}] status={} author={} {}",
                note.created_at,
                note.status,
                note.author_name.unwrap_or_else(|| "system".to_string()),
                note.body
            ));
        }

        Ok(())
    }

    pub(super) fn cmd_task_add(&mut self, input: TaskAddInput) -> Res<()> {
        let TaskAddInput {
            actor,
            board,
            title,
            description,
            size,
            parent,
            assignee,
            depends,
        } = input;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, &actor)?;
        let board_id = Self::resolve_board_id_on(&tx, &board)?;
        Self::require_action_on(&tx, actor_id, board_id, "create")?;

        if let Some(parent_id) = parent {
            Self::ensure_task_in_board_conn(&tx, board_id, parent_id)?;
        }

        if size == "large" && depends.is_some() {
            return Err(Box::new(AppErr(
                "dependency",
                "large task dependencies are managed by children".into(),
            )));
        }

        let assignee_id = match assignee {
            Some(name) => Some(Self::get_or_create_user_on(&tx, &name)?),
            None => None,
        };

        tx.execute("INSERT INTO tasks(board_id,title,description,size,parent_id,assignee_id,status,created_by,updated_by,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,'pending',?7,?7,?8,?8)", params![board_id, title, description, &size, parent, assignee_id, actor_id, now_secs()])?;
        let task_id = tx.last_insert_rowid();

        if let Some(dep_ids) = depends
            && size != "large"
        {
            Self::set_dependencies(&tx, board_id, task_id, &dep_ids)?;
        }

        if size == "large" {
            let parent_of_task = Self::sync_large_task(&tx, board_id, task_id)?;
            Self::sync_large_ancestors(&tx, board_id, parent_of_task)?;
        }
        Self::sync_large_ancestors(&tx, board_id, parent)?;

        tx.commit()?;

        self.emit_line(format!("created task {}", task_id));
        Ok(())
    }

    pub(super) fn cmd_task_edit(&mut self, input: TaskEditInput) -> Res<()> {
        let TaskEditInput {
            actor,
            board,
            task_id,
            title,
            description,
            size,
            parent,
            assignee,
            depends,
        } = input;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, &actor)?;
        let board_id = Self::resolve_board_id_on(&tx, &board)?;
        let old = Self::get_task_on(&tx, board_id, task_id)?;
        Self::require_action_on(&tx, actor_id, board_id, "update")?;

        let old_parent = old.parent_id;
        let old_size = old.size.clone();
        let old_assignee_id = old.assignee_id;
        let old_title = old.title;
        let old_description = old.description;

        let new_parent = parent.or(old_parent);
        Self::ensure_parent_assignment_conn(&tx, board_id, task_id, new_parent)?;

        let new_size = size.unwrap_or(old_size.clone());
        if new_size == "large" && depends.is_some() {
            return Err(Box::new(AppErr(
                "dependency",
                "large task dependencies are managed by children".into(),
            )));
        }

        let assignee_id = match assignee {
            Some(name) if !name.trim().is_empty() => Some(Self::get_or_create_user_on(&tx, &name)?),
            Some(_) => None,
            None => old_assignee_id,
        };

        tx.execute(
            "UPDATE tasks SET title=?1,description=?2,size=?3,parent_id=?4,assignee_id=?5,updated_by=?6,updated_at=?7 WHERE id=?8 AND board_id=?9",
            params![
                title.unwrap_or(old_title),
                description.unwrap_or(old_description),
                &new_size,
                new_parent,
                assignee_id,
                actor_id,
                now_secs(),
                task_id,
                board_id
            ],
        )?;

        if let Some(dep_ids) = depends
            && new_size != "large"
        {
            Self::set_dependencies(&tx, board_id, task_id, &dep_ids)?;
        }

        if new_size == "large" {
            let parent_of_task = Self::sync_large_task(&tx, board_id, task_id)?;
            Self::sync_large_ancestors(&tx, board_id, parent_of_task)?;
        }

        if old_parent != new_parent {
            Self::sync_large_ancestors(&tx, board_id, old_parent)?;
        }
        Self::sync_large_ancestors(&tx, board_id, new_parent)?;

        tx.commit()?;

        self.emit_line(format!("updated task {}", task_id));
        Ok(())
    }

    pub(super) fn cmd_task_status(
        &mut self,
        actor: &str,
        board: &str,
        task_id: i64,
        status: &str,
        note: Option<String>,
    ) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "set_status")?;
        let task = Self::get_task_on(&tx, board_id, task_id)?;

        let normalized_note = Self::normalize_note(note)?;
        if Self::status_requires_note(status) && normalized_note.is_none() {
            return Err(Box::new(AppErr(
                "input",
                format!("--note is required when status is {}", status),
            )));
        }

        if task.size == "large" && (status == "in_progress" || status == "completed") {
            return Err(Box::new(AppErr(
                "status",
                "large task status is derived from children and cannot be set to in_progress/completed manually"
                    .into(),
            )));
        }

        Self::ensure_dependencies_completed(&tx, task_id, status)?;

        tx.execute(
            "UPDATE tasks SET status=?1,updated_by=?2,updated_at=?3 WHERE id=?4 AND board_id=?5",
            params![status, actor_id, now_secs(), task_id, board_id],
        )?;

        if let Some(note_body) = normalized_note {
            Self::insert_post_note(&tx, task_id, Some(actor_id), status, &note_body)?;
        }

        Self::sync_large_ancestors(&tx, board_id, task.parent_id)?;

        tx.commit()?;
        self.emit_line("status updated".to_string());
        Ok(())
    }

    pub(super) fn cmd_task_delete(&mut self, actor: &str, board: &str, task_id: i64) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "delete")?;
        let task = Self::get_task_on(&tx, board_id, task_id)?;

        let changed_rows = tx.execute(
            "DELETE FROM tasks WHERE id=?1 AND board_id=?2",
            params![task_id, board_id],
        )?;

        Self::sync_large_ancestors(&tx, board_id, task.parent_id)?;

        tx.commit()?;
        self.emit_line(format!("deleted {} row(s)", changed_rows));
        Ok(())
    }

    fn set_dependencies(
        conn: &Connection,
        board_id: i64,
        task_id: i64,
        dep_ids: &[i64],
    ) -> Res<()> {
        let deps = normalize_depends(dep_ids.to_vec());
        for dep in &deps {
            if *dep == task_id {
                return Err(Box::new(AppErr(
                    "dependency",
                    "self dependency is not allowed".into(),
                )));
            }
            Self::ensure_task_in_board_conn(conn, board_id, *dep)?;
        }

        let mut graph = Self::dependency_graph(conn, board_id)?;
        graph.insert(task_id, deps.clone());
        for dep in &deps {
            if Self::reachable(&graph, *dep, task_id, &mut HashSet::new()) {
                return Err(Box::new(AppErr("dependency", "cycle detected".into())));
            }
        }

        conn.execute("DELETE FROM task_dependencies WHERE task_id=?1", [task_id])?;
        for dep in deps {
            conn.execute(
                "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES (?1,?2)",
                params![task_id, dep],
            )?;
        }

        Ok(())
    }

    fn dependency_graph(conn: &Connection, board_id: i64) -> Res<HashMap<i64, Vec<i64>>> {
        let mut statement = conn.prepare(
            "SELECT td.task_id,td.depends_on_task_id FROM task_dependencies td JOIN tasks t ON t.id=td.task_id WHERE t.board_id=?1",
        )?;
        let mut rows = statement.query([board_id])?;
        let mut graph = HashMap::new();
        while let Some(row) = rows.next()? {
            graph
                .entry(row.get::<_, i64>(0)?)
                .or_insert_with(Vec::new)
                .push(row.get::<_, i64>(1)?);
        }
        Ok(graph)
    }

    fn reachable(
        graph: &HashMap<i64, Vec<i64>>,
        start: i64,
        target: i64,
        seen: &mut HashSet<i64>,
    ) -> bool {
        if start == target {
            return true;
        }
        if !seen.insert(start) {
            return false;
        }
        if let Some(neighbors) = graph.get(&start) {
            for neighbor in neighbors {
                if Self::reachable(graph, *neighbor, target, seen) {
                    return true;
                }
            }
        }
        false
    }

    fn depends_of(conn: &Connection, task_id: i64) -> Res<Vec<i64>> {
        let mut statement = conn.prepare(
            "SELECT depends_on_task_id FROM task_dependencies WHERE task_id=?1 ORDER BY depends_on_task_id",
        )?;
        let mut rows = statement.query([task_id])?;
        let mut deps = Vec::new();
        while let Some(row) = rows.next()? {
            deps.push(row.get::<_, i64>(0)?);
        }
        Ok(deps)
    }

    fn notes_of(conn: &Connection, task_id: i64) -> Res<Vec<TaskNote>> {
        let mut statement = conn.prepare(
            "SELECT n.created_at,n.status,u.name,n.body FROM task_notes n LEFT JOIN users u ON u.id=n.author_id WHERE n.task_id=?1 ORDER BY n.id",
        )?;
        let mut rows = statement.query([task_id])?;
        let mut notes = Vec::new();
        while let Some(row) = rows.next()? {
            notes.push(TaskNote {
                created_at: row.get(0)?,
                status: row.get(1)?,
                author_name: row.get(2)?,
                body: row.get(3)?,
            });
        }
        Ok(notes)
    }

    fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            size: row.get(3)?,
            parent_id: row.get(4)?,
            assignee_id: row.get(5)?,
            assignee_name: row.get(6)?,
            status: row.get(7)?,
        })
    }

    fn get_task(&self, board_id: i64, task_id: i64) -> Res<Task> {
        Self::get_task_on(&self.conn, board_id, task_id)
    }

    fn get_task_on(conn: &Connection, board_id: i64, task_id: i64) -> Res<Task> {
        let mut statement = conn.prepare("SELECT t.id,t.title,t.description,t.size,t.parent_id,t.assignee_id,u.name,t.status FROM tasks t LEFT JOIN users u ON u.id=t.assignee_id WHERE t.id=?1 AND t.board_id=?2")?;
        statement
            .query_row([task_id, board_id], Self::map_task)
            .optional()?
            .ok_or_else(|| AppErr("not_found", "task not found in board".into()).into())
    }

    fn ensure_task_in_board_conn(conn: &Connection, board_id: i64, task_id: i64) -> Res<()> {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM tasks WHERE id=?1 AND board_id=?2",
                params![task_id, board_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(Box::new(AppErr(
                "dependency",
                format!("task {} not in board {}", task_id, board_id),
            )));
        }
        Ok(())
    }

    fn ensure_parent_assignment_conn(
        conn: &Connection,
        board_id: i64,
        task_id: i64,
        parent_id: Option<i64>,
    ) -> Res<()> {
        let Some(parent_id) = parent_id else {
            return Ok(());
        };

        if parent_id == task_id {
            return Err(Box::new(AppErr(
                "dependency",
                "self parent is not allowed".into(),
            )));
        }

        Self::ensure_task_in_board_conn(conn, board_id, parent_id)?;

        let mut cursor = Some(parent_id);
        let mut seen = HashSet::new();
        while let Some(current) = cursor {
            if current == task_id {
                return Err(Box::new(AppErr(
                    "dependency",
                    "parent cycle detected".into(),
                )));
            }
            if !seen.insert(current) {
                return Err(Box::new(AppErr(
                    "dependency",
                    "parent cycle detected".into(),
                )));
            }
            cursor = conn
                .query_row(
                    "SELECT parent_id FROM tasks WHERE id=?1 AND board_id=?2",
                    params![current, board_id],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .optional()?
                .flatten();
        }

        Ok(())
    }

    fn ensure_dependencies_completed(conn: &Connection, task_id: i64, status: &str) -> Res<()> {
        if status != "in_progress" && status != "completed" {
            return Ok(());
        }

        let mut statement = conn.prepare(
            "SELECT td.depends_on_task_id,t.status FROM task_dependencies td JOIN tasks t ON t.id=td.depends_on_task_id WHERE td.task_id=?1 ORDER BY td.depends_on_task_id",
        )?;
        let mut rows = statement.query([task_id])?;
        while let Some(row) = rows.next()? {
            let dependency_id: i64 = row.get(0)?;
            let dependency_status: String = row.get(1)?;
            if dependency_status != "completed" {
                return Err(Box::new(AppErr(
                    "dependency",
                    format!(
                        "task {} cannot move to {} because dependency {} is {}",
                        task_id, status, dependency_id, dependency_status
                    ),
                )));
            }
        }

        Ok(())
    }

    fn status_requires_note(status: &str) -> bool {
        status == "blocked" || status == "completed"
    }

    fn normalize_note(note: Option<String>) -> Res<Option<String>> {
        match note {
            None => Ok(None),
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(Box::new(AppErr("input", "--note cannot be empty".into())));
                }
                Ok(Some(trimmed.to_string()))
            }
        }
    }

    fn insert_post_note(
        conn: &Connection,
        task_id: i64,
        author_id: Option<i64>,
        status: &str,
        body: &str,
    ) -> Res<()> {
        conn.execute(
            "INSERT INTO task_notes(task_id,author_id,status,body,created_at) VALUES (?1,?2,?3,?4,?5)",
            params![task_id, author_id, status, body, now_secs()],
        )?;
        Ok(())
    }

    fn sync_large_ancestors(conn: &Connection, board_id: i64, mut task_id: Option<i64>) -> Res<()> {
        let mut seen = HashSet::new();
        while let Some(current) = task_id {
            if !seen.insert(current) {
                return Err(Box::new(AppErr(
                    "dependency",
                    "parent cycle detected".into(),
                )));
            }
            task_id = Self::sync_large_task(conn, board_id, current)?;
        }
        Ok(())
    }

    fn sync_large_task(conn: &Connection, board_id: i64, task_id: i64) -> Res<Option<i64>> {
        let task_meta: Option<(String, String, Option<i64>)> = conn
            .query_row(
                "SELECT size,status,parent_id FROM tasks WHERE id=?1 AND board_id=?2",
                params![task_id, board_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((size, current_status, parent_id)) = task_meta else {
            return Ok(None);
        };

        if size != "large" {
            return Ok(parent_id);
        }

        let child_ids = Self::child_ids_of(conn, board_id, task_id)?;
        Self::set_dependencies(conn, board_id, task_id, &child_ids)?;

        let new_status = Self::derive_large_status(conn, board_id, task_id)?;
        if new_status != current_status {
            conn.execute(
                "UPDATE tasks SET status=?1,updated_by=NULL,updated_at=?2 WHERE id=?3 AND board_id=?4",
                params![&new_status, now_secs(), task_id, board_id],
            )?;
            let note = format!(
                "system: auto-updated status from '{}' to '{}' based on child task states",
                current_status, new_status
            );
            Self::insert_post_note(conn, task_id, None, &new_status, &note)?;
        }

        Ok(parent_id)
    }

    fn child_ids_of(conn: &Connection, board_id: i64, parent_id: i64) -> Res<Vec<i64>> {
        let mut statement =
            conn.prepare("SELECT id FROM tasks WHERE board_id=?1 AND parent_id=?2 ORDER BY id")?;
        let mut rows = statement.query(params![board_id, parent_id])?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? {
            ids.push(row.get::<_, i64>(0)?);
        }
        Ok(ids)
    }

    fn derive_large_status(conn: &Connection, board_id: i64, task_id: i64) -> Res<String> {
        let mut statement =
            conn.prepare("SELECT status FROM tasks WHERE board_id=?1 AND parent_id=?2")?;
        let mut rows = statement.query(params![board_id, task_id])?;

        let mut has_children = false;
        let mut all_completed = true;
        let mut has_blocked = false;
        let mut has_in_progress = false;
        let mut has_completed = false;

        while let Some(row) = rows.next()? {
            has_children = true;
            let child_status: String = row.get(0)?;
            match child_status.as_str() {
                "completed" => {
                    has_completed = true;
                }
                "in_progress" => {
                    has_in_progress = true;
                    all_completed = false;
                }
                "blocked" => {
                    has_blocked = true;
                    all_completed = false;
                }
                _ => {
                    all_completed = false;
                }
            }
        }

        if !has_children {
            return Ok("pending".to_string());
        }
        if all_completed {
            return Ok("completed".to_string());
        }
        if has_blocked {
            return Ok("blocked".to_string());
        }
        if has_in_progress || has_completed {
            return Ok("in_progress".to_string());
        }

        Ok("pending".to_string())
    }
}
