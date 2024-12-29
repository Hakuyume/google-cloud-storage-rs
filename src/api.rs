mod future;
pub mod json;
pub mod xml;

use bytes::Bytes;

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

    #[error("Api({0:?})")]
    Api(http::Response<Bytes>),
}

#[cfg(test)]
mod tests;
