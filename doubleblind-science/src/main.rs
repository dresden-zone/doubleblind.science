use std::time::Duration;

use axum::response::Response;
use axum::Server;
use clap::Parser;

use tokio::select;
use tower_http::cors::CorsLayer;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{error, info, Level, Span};
use tracing_subscriber::FmtSubscriber;

use crate::args::DoubleBlindArgs;
use crate::routes::route;
use crate::state::DoubleBlindState;

mod args;
mod auth;
mod routes;
pub mod service;
mod state;
pub mod structs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = DoubleBlindArgs::parse();

  let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::TRACE)
    .compact()
    .finish();

  tracing::subscriber::set_global_default(subscriber)?;

  info!(concat!(
    "Booting ",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    "..."
  ));

  let cors = CorsLayer::very_permissive();

  let state = DoubleBlindState::new(
    &args.database_username,
    &args.database_password_file,
    &args.database_host,
    &args.database_name,
    &args.github_client_id,
    &args.website_path,
    &args.website_domain,
    &args.github_hmac_secret_file,
    &args.github_secret_key_file,
  )
  .await;

  let deployment_service_copy = state.deployment_service.clone();
  let deploy_loop_future = deployment_service_copy.deploy_loop();

  let router = route()
    .layer(cors)
    .layer(
      TraceLayer::new_for_http()
        .on_request(|request: &hyper::Request<axum::body::Body>, _: &'_ _| {
          info!(
            "URI: {:?} METHOD: {:?} HEADERS: {:?}",
            request.uri(),
            request.method(),
            request.headers()
          );
        })
        .on_response(|response: &Response, _latency: Duration, _span: &Span| {
          info!(
            "Success: HEADER: {:?} BODY: {:?}",
            response.headers(),
            response.body(),
          )
        })
        .on_failure(
          |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
            info!("Error: {:?}", _error)
          },
        ),
    )
    .with_state(state);
  let server = Server::bind(&args.listen_addr).serve(router.into_make_service());

  info!("Listening on http://{}...", server.local_addr());

  select! {
    result = server => {
      if let Err(e) = result {
        error!("Error while serving api: {}", e);
      }
    },
    result = deploy_loop_future => {
      if let Err(e) = result {
        error!("Error while serving api: {}", e);
      }
    }
  }

  Ok(())
}
