use axum::Server;
use clap::Parser;
use tower_http::cors::CorsLayer;
use tracing::{error, info, Level};
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
    .with_max_level(Level::INFO)
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
    &args.github_client_secret_file,
    &args.website_path,
    &args.website_domain,
    &args.github_hmac_secret_file,
  )
  .await;

  let router = route().layer(cors).with_state(state);
  let server = Server::bind(&args.listen_addr).serve(router.into_make_service());

  info!("Listening on http://{}...", server.local_addr());

  if let Err(err) = server.await {
    error!("Error while serving api: {}", err);
  }

  Ok(())
}
