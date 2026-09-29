use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::{AgentUser, AuthUser},
    email,
    error::AppError,
    models::{CreateTicket, Message, Ticket, TicketFilters, TicketWithThread, UpdateTicket},
    AppState,
};

pub async fn create_ticket(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateTicket>,
) -> Result<Json<Ticket>, AppError> {
    if payload.subject.trim().is_empty() || payload.body.trim().is_empty() {
        return Err(AppError::BadRequest("subject and body are required".into()));
    }

    let mut tx = state.pool.begin().await?;

    let ticket = sqlx::query_as::<_, Ticket>(
        "insert into tickets (subject, customer_id) values ($1, $2) returning *",
    )
    .bind(&payload.subject)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("insert into messages (ticket_id, author_id, body) values ($1, $2, $3)")
        .bind(ticket.id)
        .bind(user.id)
        .bind(&payload.body)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let agent_emails: Vec<String> = sqlx::query_scalar(
    "select email from profiles where role = 'agent'",
)
.fetch_all(&state.pool)
.await?;

email::send_new_ticket_email(&state.config, agent_emails, ticket.id, &ticket.subject, &payload.body);
    Ok(Json(ticket))
    
}

pub async fn list_tickets(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(filters): Query<TicketFilters>,
) -> Result<Json<Vec<Ticket>>, AppError> {
    let is_agent = user.role == "agent";

    let tickets = sqlx::query_as::<_, Ticket>(
        r#"
        select * from tickets
        where ($1 or customer_id = $2)
          and ($3::text is null or status = $3)
          and ($4::text is null or priority = $4)
          and ($5::text is null or subject ilike '%' || $5 || '%')
        order by updated_at desc
        "#,
    )
    .bind(is_agent)
    .bind(user.id)
    .bind(filters.status)
    .bind(filters.priority)
    .bind(filters.q)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(tickets))
}

pub async fn get_ticket(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TicketWithThread>, AppError> {
    let ticket = sqlx::query_as::<_, Ticket>("select * from tickets where id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let is_agent = user.role == "agent";
    if !is_agent && ticket.customer_id != user.id {
        return Err(AppError::Forbidden);
    }

    let messages = sqlx::query_as::<_, Message>(
        "select * from messages where ticket_id = $1 and ($2 or not is_internal) order by created_at asc",
    )
    .bind(id)
    .bind(is_agent)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(TicketWithThread { ticket, messages }))
}

pub async fn update_ticket(
    State(state): State<AppState>,
    AgentUser(_agent): AgentUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTicket>,
) -> Result<Json<Ticket>, AppError> {
    let ticket = sqlx::query_as::<_, Ticket>(
        r#"
        update tickets set
            status = coalesce($1, status),
            priority = coalesce($2, priority),
            assignee_id = coalesce($3, assignee_id),
            updated_at = now()
        where id = $4
        returning *
        "#,
    )
    .bind(payload.status)
    .bind(payload.priority)
    .bind(payload.assignee_id)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(ticket))
}