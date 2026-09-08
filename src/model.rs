use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Inbox,
    Ongoing,
}

impl ColumnType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColumnType::Inbox => "inbox",
            ColumnType::Ongoing => "ongoing",
        }
    }

    // Infallible on purpose: an unrecognised column falls back to Inbox rather
    // than dropping the row, so a hand-edited database still opens.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "ongoing" => ColumnType::Ongoing,
            _ => ColumnType::Inbox,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub column: ColumnType,
    pub is_priority: bool,
    pub is_completed: bool,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}
