// https://cloud.google.com/storage/docs/xml-api/put-object-upload

use super::super::future::{oneshot, Oneshot};
use super::super::Error;
use headers::{ContentLength, ContentType, HeaderMapExt};
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
        content_length: None,
        content_type: None,
        content_md5: None,
    }
}

pub struct Builder<T> {
    bucket_name: String,
    object_name: String,
    body: T,
    content_length: Option<u64>,
    content_type: Option<mime::Mime>,
    content_md5: Option<[u8; 16]>,
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
            content_length,
            content_type,
            content_md5,
        } = self;
        let mut builder = http::Request::put(super::uri(bucket_name, object_name));
        if let Some(headers) = builder.headers_mut() {
            if let Some(content_length) = content_length {
                headers.typed_insert(ContentLength(content_length));
            }
            if let Some(content_type) = content_type {
                headers.typed_insert(ContentType::from(content_type));
            }
            headers.typed_insert(crate::header::XGoogHash {
                crc32c: None,
                md5: content_md5,
            });
        }
        let request = builder.body(body).map_err(Error::Http);
        Future(oneshot(service, request))
    }

    pub fn content_length(mut self, value: u64) -> Self {
        self.content_length = Some(value);
        self
    }

    pub fn content_type(mut self, value: mime::Mime) -> Self {
        self.content_type = Some(value);
        self
    }

    pub fn content_md5(mut self, value: [u8; 16]) -> Self {
        self.content_md5 = Some(value);
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
