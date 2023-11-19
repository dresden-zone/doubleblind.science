#[cfg(feature = "hickory-proto")]
mod error;
#[cfg(feature = "hickory-proto")]
mod try_from;

mod models;

#[cfg(feature = "hickory-proto")]
pub use error::EntityError;
pub use models::*;
#[cfg(feature = "hickory-proto")]
pub use try_from::IntoRecord;
