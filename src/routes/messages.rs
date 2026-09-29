use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    email,
    error::AppError,
    models::{CreateMessage, Message, Profile, Ticket},
    AppState,
};

pub async fn create_message(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(ticket_id): Path<Uuid>,
    Json(payload): Json<CreateMessage>,
) -> Result<Json<Message>, AppError> {
    if payload.body.trim().is_empty() {
        return Err(AppError::BadRequest("body is required".into()));
    }

    let ticket = sqlx::query_as::<_, Ticket>("select * from tickets where id = $1")
        .bind(ticket_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let is_agent = user.role == "agent";
    if !is_agent && ticket.customer_id != user.id {
        return Err(AppError::Forbidden);
    }

    // Only agents can leave internal notes; force false for customers.
    let is_internal = is_agent && payload.is_internal;

    let message = sqlx::query_as::<_, Message>(
        "insert into messages (ticket_id, author_id, body, is_internal) values ($1, $2, $3, $4) returning *",
    )
    .bind(ticket_id)
    .bind(user.id)
    .bind(&payload.body)
    .bind(is_internal)
    .fetch_one(&state.pool)
    .await?;

    if is_agent && !is_internal {
        let customer = sqlx::query_as::<_, Profile>("select * from profiles where id = $1")
            .bind(ticket.customer_id)
            .fetch_optional(&state.pool)
            .await?;

        if let Some(customer) = customer {
            email::send_reply_email(&state.config, &customer.email, &ticket.subject, &payload.body);
        }
    }

    Ok(Json(message))
}