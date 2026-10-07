use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::Jwk};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, error::AppError, models::Profile};

#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
}

/// Any authenticated user (customer or agent). Extracts the profile row.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Profile);

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let jwt_header = decode_header(token).map_err(|e| {
            tracing::error!(error = ?e, "jwt header decode failed");
            AppError::Unauthorized
        })?;

        let kid = jwt_header.kid.ok_or(AppError::Unauthorized)?;

        let jwk: &Jwk = app_state.jwks.get(&kid).ok_or_else(|| {
            tracing::error!(kid, "no matching jwk for kid");
            AppError::Unauthorized
        })?;

        let decoding_key = DecodingKey::from_jwk(jwk).map_err(|e| {
            tracing::error!(error = ?e, "failed to build decoding key from jwk");
            AppError::Unauthorized
        })?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&["authenticated"]);

        let claims = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| {
                tracing::error!(error = ?e, "jwt decode failed");
                AppError::Unauthorized
            })?
            .claims;

        let profile = sqlx::query_as::<_, Profile>(
            "select id, email, name, role from profiles where id = $1",
        )
        .bind(claims.sub)
        .fetch_optional(&app_state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

        Ok(AuthUser(profile))
    }
}

/// Authenticated user who must have the 'agent' role.
#[derive(Debug, Clone)]
pub struct AgentUser(pub Profile);

impl<S> FromRequestParts<S> for AgentUser
where
    AppState: FromRef<S>,
    S: Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(profile) = AuthUser::from_request_parts(parts, state).await?;
        if profile.role != "agent" {
            return Err(AppError::Forbidden);
        }
        Ok(AgentUser(profile))
    }
}
