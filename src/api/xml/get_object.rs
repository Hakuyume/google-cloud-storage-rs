// https://cloud.google.com/storage/docs/xml-api/get-object-download

use http::{Request, Response};
use http_body::Body;
use tower::Service;

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
        S: Service<Request<T>, Response = Response<U>>,
        T: Default,
        U: Body,
    {
        let Self {
            bucket_name,
            object_name,
        } = self;
        super::send(
            service,
            Request::get(super::uri(bucket_name, object_name)),
            T::default(),
        )
    }
}
pub type Future<S, T, U> = super::Send<S, T, U>;
