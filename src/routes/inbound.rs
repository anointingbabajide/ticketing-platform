use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, email, models::Ticket};

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    data: EventData,
}

#[derive(Deserialize)]
struct EventData {
    email_id: String, // confirm the field name against a logged payload
}

#[derive(Deserialize)]
struct Received {
    from: String,
    subject: Option<String>,
    message_id: String,
    text: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
}

pub async fn inbound_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let Ok(wh) = svix::webhooks::Webhook::new(&state.config.resend_webhook_secret) else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    if wh.verify(&body, &headers).is_err() {
        return StatusCode::UNAUTHORIZED;
    }
    match process(&state, &body).await {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            tracing::error!(error = ?e, "inbound email failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn process(state: &AppState, body: &[u8]) -> anyhow::Result<()> {
    let event: Event = serde_json::from_slice(body)?;
    if event.kind != "email.received" {
        return Ok(());
    }

    let mail: Received = reqwest::Client::new()
        .get(format!(
            "https://api.resend.com/emails/receiving/{}",
            event.data.email_id
        ))
        .bearer_auth(&state.config.resend_api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let hdr = |name: &str| {
        mail.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };

    let sender = bare_address(&mail.from).to_lowercase();

    // loop guard: auto replies and our own address
    let auto = hdr("auto-submitted").map_or(false, |v| !v.eq_ignore_ascii_case("no"));
    if auto || sender == bare_address(&state.config.from_email).to_lowercase() {
        return Ok(());
    }

    let text = strip_quoted(mail.text.as_deref().unwrap_or(""));
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let subject = mail
        .subject
        .clone()
        .unwrap_or_else(|| "(no subject)".into());

    // dedupe on the email Message-ID
    let seen: bool =
        sqlx::query_scalar("select exists(select 1 from messages where email_message_id = $1)")
            .bind(&mail.message_id)
            .fetch_one(&state.pool)
            .await?;
    if seen {
        return Ok(());
    }

    // 1) match by In-Reply-To / References
    let refs: Vec<String> = [hdr("in-reply-to"), hdr("references")]
        .into_iter()
        .flatten()
        .flat_map(|v| v.split_whitespace().map(str::to_string))
        .collect();

    let mut ticket: Option<Ticket> = None;
    if !refs.is_empty() {
        ticket = sqlx::query_as::<_, Ticket>(
            "select t.* from tickets t join messages m on m.ticket_id = t.id where m.email_message_id = any($1) limit 1",
        )
        .bind(&refs)
        .fetch_optional(&state.pool)
        .await?;
    }

    // 2) fall back to the [#abcd1234] tag, only if the sender owns that ticket
    if ticket.is_none() {
        if let Some(short) = tag_in(&subject) {
            ticket = sqlx::query_as::<_, Ticket>(
                r#"select * from tickets
                   where id::text like $1 || '%'
                     and (lower(customer_email) = $2
                          or customer_id in (select id from profiles where lower(email) = $2))
                   limit 1"#,
            )
            .bind(short)
            .bind(&sender)
            .fetch_optional(&state.pool)
            .await?;
        }
    }

    match ticket {
        Some(t) => {
            sqlx::query(
                "insert into messages (ticket_id, author_id, author_email, body, email_message_id) values ($1, $2, $3, $4, $5)",
            )
            .bind(t.id)
            .bind(t.customer_id)
            .bind(&sender)
            .bind(text)
            .bind(&mail.message_id)
            .execute(&state.pool)
            .await?;

            sqlx::query("update tickets set status = 'open' where id = $1 and status = 'resolved'")
                .bind(t.id)
                .execute(&state.pool)
                .await?;

            let agent_emails: Vec<String> = match t.assignee_id {
                Some(aid) => {
                    sqlx::query_scalar("select email from profiles where id = $1")
                        .bind(aid)
                        .fetch_all(&state.pool)
                        .await?
                }
                None => {
                    sqlx::query_scalar("select email from profiles where role = 'agent'")
                        .fetch_all(&state.pool)
                        .await?
                }
            };
            email::send_customer_reply_email(&state.config, agent_emails, t.id, &t.subject, text);
        }
        None => {
            // link to an existing portal account if the email matches one
            let profile_id: Option<Uuid> =
                sqlx::query_scalar("select id from profiles where lower(email) = $1")
                    .bind(&sender)
                    .fetch_optional(&state.pool)
                    .await?;

            let mut tx = state.pool.begin().await?;

            let t = sqlx::query_as::<_, Ticket>(
                "insert into tickets (subject, customer_id, customer_email, source) values ($1, $2, $3, 'email') returning *",
            )
            .bind(&subject)
            .bind(profile_id)
            .bind(&sender)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                "insert into messages (ticket_id, author_id, author_email, body, email_message_id) values ($1, $2, $3, $4, $5)",
            )
            .bind(t.id)
            .bind(profile_id)
            .bind(&sender)
            .bind(text)
            .bind(&mail.message_id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            let agent_emails: Vec<String> =
                sqlx::query_scalar("select email from profiles where role = 'agent'")
                    .fetch_all(&state.pool)
                    .await?;
            email::send_new_ticket_email(&state.config, agent_emails, t.id, &t.subject, text);
        }
    }

    Ok(())
}

fn bare_address(s: &str) -> &str {
    match (s.rfind('<'), s.rfind('>')) {
        (Some(a), Some(b)) if a < b => &s[a + 1..b],
        _ => s.trim(),
    }
}

fn tag_in(subject: &str) -> Option<String> {
    let start = subject.find("[#")? + 2;
    let end = subject[start..].find(']')? + start;
    Some(subject[start..end].to_lowercase())
}

fn strip_quoted(text: &str) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim_start();
        if l.starts_with('>') || (l.starts_with("On ") && l.trim_end().ends_with("wrote:")) {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}
