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
    pub(super) parent: Option<i64>,
    pub(super) assignee: Option<String>,
    pub(super) depends: Option<Vec<i64>>,
}

impl App {
    pub(super) fn cmd_task_list(
        &self,
        actor: &str,
        board: &str,
        status: Option<&str>,
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
        let mut statement = self.conn.prepare("SELECT t.id,t.title,t.description,t.parent_id,t.assignee_id,u.name,t.status FROM tasks t LEFT JOIN users u ON u.id=t.assignee_id WHERE t.board_id=?1 ORDER BY t.id")?;
        let mut rows = statement.query([board_id])?;

        self.emit_line(format!(
            "{:<4} {:<12} {:<7} {:<12} {:<12} TITLE",
            "ID", "STATUS", "PARENT", "ASSIGNEE", "dependsOn"
        ));
        while let Some(row) = rows.next()? {
            let task = Self::map_task(row)?;
            if let Some(value) = status
                && task.status != value {
                    continue;
                }
            if let Some(value) = parent
                && task.parent_id != Some(value) {
                    continue;
                }
            if let Some(value) = assignee_id
                && task.assignee_id != Some(value) {
                    continue;
                }

            self.emit_line(format!(
                "{:<4} {:<12} {:<7} {:<12} {:<12} {}",
                task.id,
                task.status,
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

        Ok(())
    }

    pub(super) fn cmd_task_add(&mut self, input: TaskAddInput) -> Res<()> {
        let TaskAddInput {
            actor,
            board,
            title,
            description,
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

        let assignee_id = match assignee {
            Some(name) => Some(Self::get_or_create_user_on(&tx, &name)?),
            None => None,
        };

        tx.execute("INSERT INTO tasks(board_id,title,description,parent_id,assignee_id,status,created_by,updated_by,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,'pending',?6,?6,?7,?7)", params![board_id, title, description, parent, assignee_id, actor_id, now_secs()])?;
        let task_id = tx.last_insert_rowid();
        if let Some(dep_ids) = depends {
            Self::set_dependencies(&tx, board_id, task_id, &dep_ids)?;
        }
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

        if let Some(parent_id) = parent {
            Self::ensure_task_in_board_conn(&tx, board_id, parent_id)?;
        }

        let assignee_id = match assignee {
            Some(name) if !name.trim().is_empty() => Some(Self::get_or_create_user_on(&tx, &name)?),
            Some(_) => None,
            None => old.assignee_id,
        };

        tx.execute(
            "UPDATE tasks SET title=?1,description=?2,parent_id=?3,assignee_id=?4,updated_by=?5,updated_at=?6 WHERE id=?7 AND board_id=?8",
            params![
                title.unwrap_or(old.title),
                description.unwrap_or(old.description),
                parent.or(old.parent_id),
                assignee_id,
                actor_id,
                now_secs(),
                task_id,
                board_id
            ],
        )?;
        if let Some(dep_ids) = depends {
            Self::set_dependencies(&tx, board_id, task_id, &dep_ids)?;
        }
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
    ) -> Res<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actor_id = Self::get_user_id_on(&tx, actor)?;
        let board_id = Self::resolve_board_id_on(&tx, board)?;
        Self::require_action_on(&tx, actor_id, board_id, "set_status")?;
        let _ = Self::get_task_on(&tx, board_id, task_id)?;

        tx.execute(
            "UPDATE tasks SET status=?1,updated_by=?2,updated_at=?3 WHERE id=?4 AND board_id=?5",
            params![status, actor_id, now_secs(), task_id, board_id],
        )?;
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
        let _ = Self::get_task_on(&tx, board_id, task_id)?;

        let changed_rows = tx.execute(
            "DELETE FROM tasks WHERE id=?1 AND board_id=?2",
            params![task_id, board_id],
        )?;
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

    fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            parent_id: row.get(3)?,
            assignee_id: row.get(4)?,
            assignee_name: row.get(5)?,
            status: row.get(6)?,
        })
    }

    fn get_task(&self, board_id: i64, task_id: i64) -> Res<Task> {
        Self::get_task_on(&self.conn, board_id, task_id)
    }

    fn get_task_on(conn: &Connection, board_id: i64, task_id: i64) -> Res<Task> {
        let mut statement = conn.prepare("SELECT t.id,t.title,t.description,t.parent_id,t.assignee_id,u.name,t.status FROM tasks t LEFT JOIN users u ON u.id=t.assignee_id WHERE t.id=?1 AND t.board_id=?2")?;
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
}
