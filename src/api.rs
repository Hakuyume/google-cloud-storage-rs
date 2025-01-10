pub mod json;
pub mod xml;

pub use http_extra::check_status::StatusError;

#[derive(Debug, thiserror::Error)]
pub enum Error<S, B> {
    #[error(transparent)]
    Body(B),
    #[error(transparent)]
    Http(http::Error),
    #[error(transparent)]
    Json(serde_json::Error),
    #[error(transparent)]
    Service(S),
    #[error(transparent)]
    Status(StatusError),
}

#[cfg(test)]
mod tests;
