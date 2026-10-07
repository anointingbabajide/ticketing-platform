use jsonwebtoken::jwk::JwkSet;
use std::collections::HashMap;

pub async fn fetch_jwks(supabase_url: &str) -> anyhow::Result<JwkSet> {
    let url = format!("{}/auth/v1/.well-known/jwks.json", supabase_url);
    let jwks: JwkSet = reqwest::get(&url).await?.json().await?;
    Ok(jwks)
}

/// Index by `kid` so verification is an O(1) lookup instead of a scan per request.
pub fn index_by_kid(jwks: &JwkSet) -> HashMap<String, jsonwebtoken::jwk::Jwk> {
    jwks.keys
        .iter()
        .filter_map(|k| k.common.key_id.clone().map(|kid| (kid, k.clone())))
        .collect()
}
