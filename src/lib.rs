pub mod api;
pub mod header;

use bytes::Bytes;

#[derive(Debug, thiserror::Error)]
pub enum Error<S, B> {
    #[error(transparent)]
    Http(http::Error),
    #[error(transparent)]
    Service(S),
    #[error(transparent)]
    Body(B),

    Status(http::Response<Bytes>),
}
