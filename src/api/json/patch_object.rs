// https://cloud.google.com/storage/docs/json_api/v1/objects/patch

use super::super::future::{collect, oneshot, Collect, Oneshot};
use super::super::Error;
use headers::{ContentType, HeaderMapExt};
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
        request: Request::default(),
    }
}

pub struct Builder {
    bucket_name: String,
    object_name: String,
    request: Request,
}

impl Builder {
    pub fn send<S, T, U>(self, service: S) -> Future<S, T, U>
    where
        S: tower::Service<http::Request<T>, Response = http::Response<U>>,
        T: From<String>,
        U: http_body::Body,
    {
        let Self {
            bucket_name,
            object_name,
            request,
        } = self;
        let mut builder = http::Request::patch(super::uri(bucket_name, object_name));
        if let Some(headers) = builder.headers_mut() {
            headers.typed_insert(ContentType::json());
        }
        let body = serde_json::to_string(&request).map_err(Error::Json);
        let request = body.and_then(|body| builder.body(body.into()).map_err(Error::Http));
        Future(collect(oneshot(service, request)))
    }

    pub fn content_type(mut self, value: mime::Mime) -> Self {
        self.request.content_type = Some(value);
        self
    }
}

#[pin_project::pin_project]
pub struct Future<S, T, U>(#[pin] Collect<Oneshot<S, T, U>, U>)
where
    S: tower::Service<http::Request<T>>,
    U: http_body::Body;
impl<S, T, U> future::Future for Future<S, T, U>
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    U: http_body::Body,
{
    type Output = Result<http::Response<Response>, Error<S::Error, U::Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx).map(|response| {
            let (parts, body) = response?.into_parts();
            let body = serde_json::from_slice(&body).map_err(Error::Json)?;
            Ok(http::Response::from_parts(parts, body))
        })
    }
}

#[serde_with::serde_as]
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<mime::Mime>,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub bucket: String,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub content_type: mime::Mime,
    #[serde_as(as = "serde_with::base64::Base64")]
    pub crc32c: [u8; 4],
    pub id: String,
    #[serde_as(as = "Option<serde_with::base64::Base64>")]
    pub md5_hash: Option<[u8; 16]>,
    pub name: String,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub size: u64,
}
