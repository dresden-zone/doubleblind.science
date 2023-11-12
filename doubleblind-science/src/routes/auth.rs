use oauth2::basic::BasicClient;

// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use axum_extra::extract::cookie::{CookieJar, Cookie};
use std::env;
use std::os::linux::raw::stat;
use std::str::FromStr;
use axum::extract::{State, Query};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;
use crate::state::DoubleBlindState;


#[derive(Deserialize)]
pub struct AuthCall {
    code: String,
    state: String
}

#[derive(Serialize)]
pub struct ReturnUrl {
    url: Url
}


pub(crate) async fn auth_login_github(
    State(mut state): State<DoubleBlindState>,
    jar: CookieJar
) -> Json<ReturnUrl> {

    let (authorize_url, csrf_state) = state.oauth_github_client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .add_scope(Scope::new("public_repo".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .add_scope(Scope::new("admin:repo_hook".to_string()))
        .url();

    let session_id = Uuid::new_v4();
    
    state.csrf_state.insert(session_id, csrf_state);

    let _ = jar.add(Cookie::new("session_id", session_id.to_string()));

    Json(ReturnUrl { url : authorize_url })
}


pub(crate) async fn auth_login_github_callback (
    State(mut state): State<DoubleBlindState>,
    Query(query): Query<AuthCall>,
    jar: CookieJar
) -> StatusCode {
    if let Some(session_cookie) = jar.get("session_id") {

        let session_id = match Uuid::from_str(session_cookie.value()) {
            Ok(value) => value,
            Err(_) => {
                return StatusCode::BAD_REQUEST;
            }
        };

        return if let Some(token) = state.csrf_state.get(&session_id) {
            let code = AuthorizationCode::new(query.code);
            if token.secret() == code.secret() {
                match state.oauth_github_client
                    .exchange_code(code)
                    .request_async(async_http_client)
                    .await {
                    Ok(token) => {
                        println!("token for scopes {:?}", token.scopes());
                        state.github_tokens.insert(session_id, token.access_token().clone());
                        state.csrf_state.remove(&session_id);
                        StatusCode::OK
                    },
                    Err(e) => {
                        StatusCode::BAD_REQUEST
                    }
                }
            } else {
                StatusCode::UNAUTHORIZED
            }
        } else {
            StatusCode::NOT_ACCEPTABLE
        }
    } else {
        StatusCode::BAD_REQUEST
    }
}
