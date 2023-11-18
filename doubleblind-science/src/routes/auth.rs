use oauth2::basic::BasicClient;

// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use crate::state::DoubleBlindState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use axum::response::{IntoResponse, Redirect};
use time::Duration;
use tracing::error;
use tracing::log::debug;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthCall {
    code: String,
    state: String,
}

#[derive(Serialize)]
pub struct ReturnUrl {
    url: Url,
}

pub(crate) async fn auth_login_github(
    State(mut state): State<DoubleBlindState>,
    jar: CookieJar,
) -> impl IntoResponse {

    let (authorize_url, csrf_state) = state.oauth_github_client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .add_scope(Scope::new("public_repo".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .add_scope(Scope::new("admin:repo_hook".to_string()))
        .url();

    let session_id = Uuid::new_v4();

    state.csrf_state.lock().await.insert(session_id, csrf_state);
    // Build the cookie

    let cookie = Cookie::build("session_id", session_id.to_string())
        .domain("api.science.tanneberger.me")
        .same_site(SameSite::Lax)
        .path("/auth")
        .secure(true)
        .http_only(true)
        .max_age(Duration::minutes(30))
        .finish();

    (jar.add(cookie), Redirect::to(authorize_url.as_ref()))
}

pub(crate) async fn auth_login_github_callback(
    State(mut state): State<DoubleBlindState>,
    Query(query): Query<AuthCall>,
    jar: CookieJar,
) -> StatusCode {
    println!("request ....");
    if let Some(session_cookie) = jar.get("session_id") {
        println!("found cookie ...");
        let session_id = match Uuid::from_str(session_cookie.value()) {
            Ok(value) => value,
            Err(e) => {
                println!("cannot parse session uuid from cookie {:?}", e);
                return StatusCode::BAD_REQUEST;
            }
        };

        println!("debug {:?}", &state.csrf_state);

        return if let Some(token) = state.csrf_state.lock().await.remove(&session_id) {
            let code = AuthorizationCode::new(query.code);
            println!("{:?} {:?} {:?}", code.secret(), token.secret(), query.state);
            if &query.state == token.secret() {
                println!("secrets match! ...");
                match state
                    .oauth_github_client
                    .exchange_code(code)
                    .request_async(async_http_client)
                    .await
                {
                    Ok(token) => {
                        println!("token for scopes {:?}", token.scopes());
                        // TOOD: database
                        //state
                        //    .github_tokens
                        //    .insert(session_id, token.access_token().clone());
                        StatusCode::OK
                    }
                    Err(e) => {
                        println!("error when trying ot fetch github token {:?}", e);
                        StatusCode::BAD_REQUEST
                    },
                }
            } else {
                StatusCode::UNAUTHORIZED
            }
        } else {
            StatusCode::UNAUTHORIZED
        };
    } else {
        StatusCode::BAD_REQUEST
    }
}
