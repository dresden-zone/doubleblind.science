use oauth2::basic::BasicClient;

// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use std::env;
use std::os::linux::raw::stat;
use axum::extract::{State, Query};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;
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
) -> Json<ReturnUrl> {

    let (authorize_url, csrf_state) = state.oauth_github_client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .add_scope(Scope::new("public_repo".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .add_scope(Scope::new("admin:repo_hook".to_string()))
        .url();

    state.csrf_state.push(csrf_state);

    Json(ReturnUrl { url : authorize_url })
}


pub(crate) async fn auth_login_github_callback (
    State(state): State<DoubleBlindState>,
    Query(query): Query<AuthCall>
) -> StatusCode {

    let code = AuthorizationCode::new(query.code);
    let csrf_token = CsrfToken::new(query.state);

    StatusCode::OK
}
