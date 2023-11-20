use std::collections::HashMap;
use oauth2::basic::BasicClient;
use std::os::linux::raw::stat;

// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use crate::structs::{GithubUser, GithubUserInfo};
use crate::state::DoubleBlindState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use reqwest::Response;
use time::Duration;
use tracing::error;
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
    let (authorize_url, csrf_state) = state
        .oauth_github_client
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
) -> impl IntoResponse {
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

        return if let Some(token) = state.csrf_state.lock().await.remove(&session_id) {
            let code = AuthorizationCode::new(query.code.clone());
            if &query.state == token.secret() {
                println!("{:?} {:?}", &query.code, &query.state);
                match state
                    .oauth_github_client
                    .exchange_code(code)
                    .request_async(async_http_client)
                    .await
                {
                    Ok(token) => {
                        println!("token for scopes {:?}", token.scopes());
                        let refresh_token = token.access_token().secret().clone();
                        println!("token {:?}", token);

                        let client = reqwest::Client::new();
                        let res: GithubUserInfo = match client
                            .get("https://api.github.com/me")
                            .header("Accept", "application/vnd.github+json")
                            .header("Authorization", refresh_token.clone())
                            .header("X-GitHub-Api-Version", "2022-11-28")
                            .send()
                            .await {
                            Ok(value) => match value
                                .json::<GithubUserInfo>()
                                .await {
                                Ok(parsed_json) => parsed_json,
                                Err(e) => {
                                    error!("cannot parse request body from github {:?}", e);
                                    return StatusCode::INTERNAL_SERVER_ERROR;
                                }
                        }
                            Err(e) => {
                                error!("error while fetching user id from github {:?}", e);
                                return StatusCode::INTERNAL_SERVER_ERROR;
                            }
                        };

                       if let Ok(Some(user)) = state.user_service.get_user_by_github(res.id).await {
                           // update user token
                           if let Err(e) = state.user_service.update_github_token(user.id, refresh_token.clone() ).await {
                               error!("error while trying to update github refresh token {:?}", e);
                               return StatusCode::INTERNAL_SERVER_ERROR;
                           }
                       } else {
                           // create new user
                           if let Err(e) = state.user_service.create_github_user(refresh_token.clone(), res.id).await {
                               error!("error while trying to create user {:?}", e);
                               return StatusCode::INTERNAL_SERVER_ERROR;
                           }
                       }

                        StatusCode::OK
                    }
                    Err(e) => {
                        println!("error when trying ot fetch github token {:?}", e);
                        StatusCode::BAD_REQUEST
                    }
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
