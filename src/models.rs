#[derive(Debug)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub parent_id: Option<i64>,
    pub assignee_id: Option<i64>,
    pub assignee_name: Option<String>,
    pub status: String,
}
