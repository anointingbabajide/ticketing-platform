use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Profile {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Ticket {
    pub id: Uuid,
    pub subject: String,
    pub status: String,
    pub priority: String,
    pub customer_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTicket {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicket {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessage {
    pub body: String,
    #[serde(default)]
    pub is_internal: bool,
}

#[derive(Debug, Deserialize)]
pub struct TicketFilters {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TicketWithThread {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub messages: Vec<Message>,
}