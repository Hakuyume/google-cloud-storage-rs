// https://cloud.google.com/storage/docs/json_api/v1/objects/patch

pub fn builder<B, O>(bucket_name: B, object_name: O, request: Request) -> Builder
where
    B: Into<String>,
    O: Into<String>,
{
    Builder {
        bucket_name: bucket_name.into(),
        object_name: object_name.into(),
        request,
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
        let builder = http::Request::patch(super::uri(bucket_name, object_name));
        super::send(service, builder, request)
    }
}
pub type Future<S, T, U> = super::Send<S, T, U, Response>;

#[serde_with::serde_as]
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<mime::Mime>,
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
