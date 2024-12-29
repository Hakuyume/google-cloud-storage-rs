// https://cloud.google.com/storage/docs/xml-api/head-object

use super::super::future::{oneshot, Oneshot};
use super::super::Error;
use bytes::Bytes;
use std::future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub fn builder<B, O>(bucket_name: B, object_name: O) -> Builder
where
    B: Into<String>,
    O: Into<String>,
{
    Builder {
        bucket_name: bucket_name.into(),
        object_name: object_name.into(),
    }
}

pub struct Builder {
    bucket_name: String,
    object_name: String,
}

impl Builder {
    pub fn send<S, T, U>(self, service: S) -> Future<S, T, U>
    where
        S: tower::Service<http::Request<T>, Response = http::Response<U>>,
        T: Default,
        U: http_body::Body,
    {
        let Self {
            bucket_name,
            object_name,
        } = self;
        let request = http::Request::head(super::uri(bucket_name, object_name))
            .body(T::default())
            .map_err(Error::Http);
        Future(oneshot(service, request))
    }
}

#[pin_project::pin_project]
pub struct Future<S, T, U>(#[pin] Oneshot<S, T, U>)
where
    S: tower::Service<http::Request<T>>,
    U: http_body::Body;
impl<S, T, U> future::Future for Future<S, T, U>
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    U: http_body::Body,
{
    type Output = Result<http::Response<()>, Error<S::Error, U::Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project()
            .0
            .poll(cx)
            .map_ok(|response| response.map(|_| ()))
    }
}
