// https://cloud.google.com/storage/docs/xml-api/put-object-upload

use headers::{Header, HeaderMapExt};
use http::{HeaderMap, Request, Response};
use http_body::Body;
use tower::Service;

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
        S: Service<Request<T>, Response = Response<U>>,
        U: Body,
    {
        let Self {
            bucket_name,
            object_name,
            body,
            headers,
        } = self;
        let mut builder = Request::put(super::uri(bucket_name, object_name));
        if let Some(h) = builder.headers_mut() {
            *h = headers;
        }
        super::empty(super::send(service, builder, body))
    }
}
pub type Future<S, T, U> = super::Empty<super::Send<S, T, U>, U>;

impl<T> Builder<T> {
    pub fn typed_header<H>(mut self, header: H) -> Self
    where
        H: Header,
    {
        self.headers.typed_insert(header);
        self
    }
}
