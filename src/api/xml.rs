pub mod delete_object;
pub mod get_object;
pub mod head_object;
pub mod put_object;

use futures::future::{Either, MapErr, MapOk};
use futures::{FutureExt, TryFutureExt};
use http::{Request, Response};
use http_body::Body;
use http_extra::check_status;
use std::fmt;
use std::future::{self, Future, Ready};
use tower::util::Oneshot;
use tower::{Service, ServiceBuilder, ServiceExt};

fn uri<B, O>(bucket_name: B, object_name: O) -> String
where
    B: fmt::Display,
    O: AsRef<[u8]>,
{
    format!(
        "https://{bucket_name}.storage.googleapis.com/{}",
        percent_encoding::percent_encode(object_name.as_ref(), percent_encoding::NON_ALPHANUMERIC),
    )
}

fn send<S, T, U>(service: S, builder: http::request::Builder, body: T) -> Send<S, T, U>
where
    S: Service<Request<T>, Response = Response<U>>,
    U: Body,
{
    let map_err: MapErrFn<S, T, U> = |value| match value {
        check_status::Error::Body(e) => Error::<S, T, U>::Body(e),
        check_status::Error::Service(e) => Error::<S, T, U>::Service(e),
        check_status::Error::Status(e) => Error::<S, T, U>::Status(e),
    };
    match builder.body(body) {
        Ok(request) => ServiceBuilder::new()
            .layer(check_status::Layer::default())
            .service(service)
            .oneshot(request)
            .map_err(map_err)
            .left_future(),
        Err(e) => future::ready(Err(Error::<S, T, U>::Http(e))).right_future(),
    }
}
type Send<S, T, U> = Either<
    MapErr<Oneshot<check_status::Service<S>, Request<T>>, MapErrFn<S, T, U>>,
    Ready<Result<Response<U>, Error<S, T, U>>>,
>;
type MapErrFn<S, T, U> = fn(
    check_status::Error<<S as Service<Request<T>>>::Error, <U as Body>::Error>,
) -> Error<S, T, U>;
type Error<S, T, U> = super::Error<<S as Service<Request<T>>>::Error, <U as Body>::Error>;

fn empty<F, B, E>(f: F) -> Empty<F, B>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    f.map_ok(|response| response.map(|_| ()))
}
type Empty<F, B> = MapOk<F, fn(Response<B>) -> Response<()>>;
