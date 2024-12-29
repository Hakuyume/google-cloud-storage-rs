// https://cloud.google.com/storage/docs/xml-api/put-object-upload

use super::super::future::{oneshot, Oneshot};
use super::super::Error;
use headers::{Header, HeaderMapExt};
use http::HeaderMap;
use std::future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub fn builder<B, O, T>(bucket_name: B, object_name: O, body: T) -> Builder<T>
where
    B: Into<String>,
    O: Into<String>,
{
    Builder {
        bucket_name: bucket_name.into(),
        object_name: object_name.into(),
        body,
        headers: HeaderMap::new(),
    }
}

pub struct Builder<T> {
    bucket_name: String,
    object_name: String,
    body: T,
    headers: HeaderMap,
}

impl<T> Builder<T> {
    pub fn send<S, U>(self, service: S) -> Future<S, T, U>
    where
        S: tower::Service<http::Request<T>, Response = http::Response<U>>,
        U: http_body::Body,
    {
        let Self {
            bucket_name,
            object_name,
            body,
            headers,
        } = self;
        let mut builder = http::Request::put(super::uri(bucket_name, object_name));
        if let Some(h) = builder.headers_mut() {
            *h = headers;
        }
        let request = builder.body(body).map_err(Error::Http);
        Future(oneshot(service, request))
    }

    pub fn header<H>(mut self, header: H) -> Self
    where
        H: Header,
    {
        self.headers.typed_insert(header);
        self
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
