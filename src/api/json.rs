pub mod patch_object;

use futures::future::{Either, MapErr};
use futures::{FutureExt, TryFutureExt};
use http::{Request, Response};
use http_body::Body;
use http_extra::{check_status, from_json, to_json};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::{self, Ready};
use tower::util::Oneshot;
use tower::{Service, ServiceBuilder, ServiceExt};

fn uri<B, O>(bucket_name: B, object_name: O) -> String
where
    B: fmt::Display,
    O: AsRef<[u8]>,
{
    format!(
        "https://storage.googleapis.com/storage/v1/b/{bucket_name}/o/{}",
        percent_encoding::percent_encode(object_name.as_ref(), percent_encoding::NON_ALPHANUMERIC),
    )
}

fn send<S, T, U, V, W>(service: S, builder: http::request::Builder, body: V) -> Send<S, T, U, W>
where
    S: Service<Request<T>, Response = Response<U>>,
    T: From<String>,
    U: Body,
    V: Serialize,
    W: for<'de> Deserialize<'de>,
{
    let map_err: MapErrFn<S, T, U> = |value| match value {
        from_json::response::Error::Service(check_status::Error::Service(e)) => {
            Error::<S, T, U>::Service(e)
        }
        from_json::response::Error::Service(check_status::Error::Body(e))
        | from_json::response::Error::Body(e) => Error::<S, T, U>::Body(e),
        from_json::response::Error::Service(check_status::Error::Status(e)) => {
            Error::<S, T, U>::Status(e)
        }
        from_json::response::Error::Json(e) => Error::<S, T, U>::Json(e),
    };
    match builder.body(body) {
        Ok(request) => match to_json::request(request) {
            Ok(request) => ServiceBuilder::new()
                .layer(from_json::response::Layer::default())
                .layer(check_status::Layer::default())
                .service(service)
                .oneshot(request)
                .map_err(map_err)
                .left_future(),
            Err(e) => future::ready(Err(Error::<S, T, U>::Json(e))).right_future(),
        },
        Err(e) => future::ready(Err(Error::<S, T, U>::Http(e))).right_future(),
    }
}
type Send<S, T, U, W> = Either<
    MapErr<
        Oneshot<from_json::response::Service<check_status::Service<S>, W>, Request<T>>,
        MapErrFn<S, T, U>,
    >,
    Ready<Result<Response<W>, Error<S, T, U>>>,
>;
type MapErrFn<S, T, U> = fn(
    from_json::response::Error<
        check_status::Error<<S as Service<Request<T>>>::Error, <U as Body>::Error>,
        <U as Body>::Error,
    >,
) -> Error<S, T, U>;
type Error<S, T, U> = super::Error<<S as Service<Request<T>>>::Error, <U as Body>::Error>;
