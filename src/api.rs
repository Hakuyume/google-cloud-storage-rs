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
    Service(S),

    Api(http::Response<Bytes>),
}
