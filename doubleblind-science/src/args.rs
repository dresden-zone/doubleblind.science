use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about)]
pub(super) struct DoubleBlindArgs {
    #[arg(
        long,
        short,
        env = "DOUBLEBLIND_LISTEN_ADDR",
        default_value = "127.0.0.1:8080"
    )]
    pub(super) listen_addr: SocketAddr,
    #[arg(long, short, env = "DOUBLEBLIND_DATABASE_HOST")]
    pub(super) database_host: String,
    #[arg(long, short, env = "DOUBLEBLIND_DATABASE_USERNAME")]
    pub(super) database_username: String,
    #[arg(long, short, env = "DOUBLEBLIND_DATABASE_PASSWORD_FILE")]
    pub(super) database_password_file: PathBuf,
    #[arg(long, short, env = "DOUBLEBLIND_DATABASE_NAME")]
    pub(super) database_name: String,
    #[arg(long, short, env = "DOUBLEBLIND_GITHUB_CLIENT_ID")]
    pub(super) github_client_id: String,
    #[arg(long, short, env = "DOUBLEBLIND_GITHUB_CLIENT_SECRET_FILE")]
    pub(super) github_client_secret_file: PathBuf,
}
