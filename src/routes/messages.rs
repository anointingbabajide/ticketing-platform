use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    email,
    error::AppError,
    models::{CreateMessage, Message, Ticket},
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
    if !is_agent && ticket.customer_id != Some(user.id) {
        return Err(AppError::Forbidden);
    }

    // Only agents can leave internal notes; force false for customers.
    let is_internal = is_agent && payload.is_internal;

    // Only agent replies that go out by email get a Message-ID.
    let will_email = is_agent && !is_internal;
    let email_message_id = will_email.then(|| format!("<{}@anoline.xyz>", Uuid::new_v4()));

    // Last stored email Message-ID on this ticket, used for threading.
    let prev_id: Option<String> = sqlx::query_scalar::<_, String>(
        "select email_message_id from messages where ticket_id = $1 and email_message_id is not null order by created_at desc limit 1",
    )
    .bind(ticket_id)
    .fetch_optional(&state.pool)
    .await?;

    let message = sqlx::query_as::<_, Message>(
        "insert into messages (ticket_id, author_id, body, is_internal, email_message_id) values ($1, $2, $3, $4, $5) returning *",
    )
    .bind(ticket_id)
    .bind(user.id)
    .bind(&payload.body)
    .bind(is_internal)
    .bind(&email_message_id)
    .fetch_one(&state.pool)
    .await?;

    if will_email {
        let to = match ticket.customer_id {
            Some(cid) => sqlx::query_scalar::<_, String>("select email from profiles where id = $1")
                .bind(cid)
                .fetch_optional(&state.pool)
                .await?,
            None => ticket.customer_email.clone(),
        };

        if let (Some(to), Some(mid)) = (to, email_message_id) {
            email::send_reply_email(
                &state.config,
                &to,
                ticket.id,
                &ticket.subject,
                &payload.body,
                mid,
                prev_id,
                ticket.customer_id.is_some(),
            );
        }
    }

    Ok(Json(message))
}