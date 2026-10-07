pub mod inbound;
pub mod messages;
pub mod tickets;

use axum::{
    Router,
    routing::{get, post},
};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tickets",
            post(tickets::create_ticket).get(tickets::list_tickets),
        )
        .route(
            "/tickets/{id}",
            get(tickets::get_ticket).patch(tickets::update_ticket),
        )
        .route("/tickets/{id}/messages", post(messages::create_message))
        .route("/webhooks/inbound-email", post(inbound::inbound_email))
}
