use crate::model::{ColumnType, Task};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => {
                // Fallback to current working directory todo.db
                let local_path = PathBuf::from("todo.db");
                Connection::open(&local_path)?
            }
        };
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn get_db_path() -> PathBuf {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("vibe-todo").join("todo.db")
        } else {
            PathBuf::from("todo.db")
        }
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                column_type TEXT NOT NULL,
                is_priority INTEGER NOT NULL DEFAULT 0,
                is_completed INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;
        // Existing databases predate the drag-and-drop ordering; seed the new
        // column from creation time so their current order is preserved.
        if self
            .conn
            .execute(
                "ALTER TABLE tasks ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .is_ok()
        {
            self.conn
                .execute("UPDATE tasks SET sort_order = -created_at", [])?;
        }
        Ok(())
    }

    /// Moves `id` into `column`, directly above `before` (or to the end when
    /// `before` is `None`), renumbering that column.
    ///
    /// The target is an anchor id rather than a position, so a row dragged
    /// downward within its own column cannot land one slot off once it is
    /// lifted out of the list.
    pub fn reposition_task(
        &self,
        id: &str,
        column: ColumnType,
        before: Option<&str>,
    ) -> Result<()> {
        // Dropping a row onto itself changes nothing. Without this the row is
        // lifted out of the list before the anchor is looked up, the anchor is
        // no longer found, and the fallback appends it to the end.
        if before == Some(id) {
            return Ok(());
        }

        let mut ids: Vec<String> = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM tasks
                 WHERE column_type = ?1 AND is_completed = 0
                 ORDER BY sort_order ASC",
            )?;
            let rows = stmt.query_map(params![column.as_str()], |row| row.get(0))?;
            rows.collect::<Result<Vec<String>>>()?
        };

        ids.retain(|existing| existing != id);
        let index = match before {
            Some(anchor) => ids
                .iter()
                .position(|existing| existing == anchor)
                .unwrap_or(ids.len()),
            None => ids.len(),
        };
        ids.insert(index, id.to_string());

        self.conn.execute(
            "UPDATE tasks SET column_type = ?1 WHERE id = ?2",
            params![column.as_str(), id],
        )?;
        for (position, task_id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                params![position as i64, task_id],
            )?;
        }
        Ok(())
    }

    pub fn insert_task(&self, title: &str, is_priority: bool, column: ColumnType) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        let is_priority_int = if is_priority { 1 } else { 0 };

        let top: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MIN(sort_order), 0) - 1 FROM tasks
                 WHERE column_type = ?1 AND is_completed = 0",
                params![column.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        self.conn.execute(
            "INSERT INTO tasks (id, title, column_type, is_priority, is_completed, created_at, sort_order)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
            params![id, title, column.as_str(), is_priority_int, now, top],
        )?;

        Ok(Task {
            id,
            title: title.to_string(),
            column,
            is_priority,
            is_completed: false,
            created_at: now,
            completed_at: None,
        })
    }

    pub fn move_task_column(&self, id: &str, target_column: ColumnType) -> Result<()> {
        self.reposition_task(id, target_column, None)
    }

    pub fn toggle_task_priority(&self, id: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT is_priority FROM tasks WHERE id = ?1")?;
        let current_priority: i32 = stmt.query_row(params![id], |row| row.get(0))?;
        let new_priority = if current_priority == 0 { 1 } else { 0 };

        self.conn.execute(
            "UPDATE tasks SET is_priority = ?1 WHERE id = ?2",
            params![new_priority, id],
        )?;
        Ok(new_priority == 1)
    }

    pub fn complete_task(&self, id: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE tasks SET is_completed = 1, completed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn restore_task(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET is_completed = 0, completed_at = NULL WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete_task(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_completed_tasks(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE is_completed = 1", [])?;
        Ok(())
    }

    pub fn get_active_tasks(&self, column: ColumnType) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, column_type, is_priority, is_completed, created_at, completed_at
             FROM tasks
             WHERE column_type = ?1 AND is_completed = 0
             ORDER BY sort_order ASC",
        )?;

        let rows = stmt.query_map(params![column.as_str()], |row| {
            let col_str: String = row.get(2)?;
            let is_priority_int: i32 = row.get(3)?;
            let is_completed_int: i32 = row.get(4)?;
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                column: ColumnType::from_str(&col_str),
                is_priority: is_priority_int == 1,
                is_completed: is_completed_int == 1,
                created_at: row.get(5)?,
                completed_at: row.get(6)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn get_completed_tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, column_type, is_priority, is_completed, created_at, completed_at
             FROM tasks
             WHERE is_completed = 1
             ORDER BY completed_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let col_str: String = row.get(2)?;
            let is_priority_int: i32 = row.get(3)?;
            let is_completed_int: i32 = row.get(4)?;
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                column: ColumnType::from_str(&col_str),
                is_priority: is_priority_int == 1,
                is_completed: is_completed_int == 1,
                created_at: row.get(5)?,
                completed_at: row.get(6)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_lifecycle() -> Result<()> {
        let db = Database::new_in_memory()?;

        // 1. Insert normal task into Inbox
        let task1 = db.insert_task("Normal task in Inbox", false, ColumnType::Inbox)?;
        assert_eq!(task1.column, ColumnType::Inbox);
        assert!(!task1.is_priority);

        // 2. Insert high priority task into Inbox
        let task2 = db.insert_task("High priority task", true, ColumnType::Inbox)?;
        assert!(task2.is_priority);

        // 3. Newest task sits at the top
        let active_inbox = db.get_active_tasks(ColumnType::Inbox)?;
        assert_eq!(active_inbox.len(), 2);
        assert_eq!(active_inbox[0].id, task2.id);
        assert_eq!(active_inbox[1].id, task1.id);

        // 3b. Hand-ordering wins over priority
        db.reposition_task(&task2.id, ColumnType::Inbox, None)?;
        let reordered = db.get_active_tasks(ColumnType::Inbox)?;
        assert_eq!(reordered[0].id, task1.id);
        assert_eq!(reordered[1].id, task2.id);

        // 3c. Dragging a row down its own column lands exactly above the anchor
        let a = db.insert_task("a", false, ColumnType::Ongoing)?;
        let b = db.insert_task("b", false, ColumnType::Ongoing)?;
        let c = db.insert_task("c", false, ColumnType::Ongoing)?;
        // insert_task puts newest on top, so the column reads c, b, a
        db.reposition_task(&c.id, ColumnType::Ongoing, Some(&a.id))?;
        let order: Vec<String> = db
            .get_active_tasks(ColumnType::Ongoing)?
            .iter()
            .map(|t| t.title.clone())
            .collect();
        assert_eq!(order, vec!["b", "c", "a"]);
        for t in [&a, &b, &c] {
            db.delete_task(&t.id)?;
        }

        // 3d. Dropping a row onto itself leaves the order untouched
        let a = db.insert_task("a", false, ColumnType::Ongoing)?;
        let b = db.insert_task("b", false, ColumnType::Ongoing)?;
        let c = db.insert_task("c", false, ColumnType::Ongoing)?;
        db.reposition_task(&b.id, ColumnType::Ongoing, Some(&b.id))?;
        let order: Vec<String> = db
            .get_active_tasks(ColumnType::Ongoing)?
            .iter()
            .map(|t| t.title.clone())
            .collect();
        assert_eq!(order, vec!["c", "b", "a"]);
        for t in [&a, &b, &c] {
            db.delete_task(&t.id)?;
        }

        // 4. Move task1 to Ongoing
        db.move_task_column(&task1.id, ColumnType::Ongoing)?;
        assert_eq!(db.get_active_tasks(ColumnType::Inbox)?.len(), 1);
        assert_eq!(db.get_active_tasks(ColumnType::Ongoing)?.len(), 1);

        // 5. Complete task1 directly from Ongoing
        db.complete_task(&task1.id)?;
        assert_eq!(db.get_active_tasks(ColumnType::Ongoing)?.len(), 0);
        assert_eq!(db.get_completed_tasks()?.len(), 1);

        // 6. Complete task2 directly from Inbox
        db.complete_task(&task2.id)?;
        assert_eq!(db.get_active_tasks(ColumnType::Inbox)?.len(), 0);
        assert_eq!(db.get_completed_tasks()?.len(), 2);

        // 7. Restore task1
        db.restore_task(&task1.id)?;
        assert_eq!(db.get_completed_tasks()?.len(), 1);
        assert_eq!(db.get_active_tasks(ColumnType::Ongoing)?.len(), 1);

        // 8. Clear completed
        db.clear_completed_tasks()?;
        assert_eq!(db.get_completed_tasks()?.len(), 0);

        Ok(())
    }
}
