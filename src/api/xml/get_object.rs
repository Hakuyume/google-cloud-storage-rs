use crate::api::Error;
use http_body_util::BodyExt;
use std::future::{self, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tower::ServiceExt;

pub fn builder<S, T>(service: S, bucket_name: &str, object_name: &str) -> Builder<S, T> {
    Builder {
        service,
        uri: super::uri(bucket_name, object_name),
        _phantom: PhantomData,
    }
}

pub struct Builder<S, T> {
    service: S,
    uri: String,
    _phantom: PhantomData<fn() -> T>,
}

impl<S, T, U> IntoFuture for Builder<S, T>
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    T: Default,
    U: http_body::Body,
{
    type Output = Result<http::Response<U>, Error<S::Error, U::Error>>;
    type IntoFuture = Future<S, T, U>;

    fn into_future(self) -> Self::IntoFuture {
        Future(State::S0(Some(self)))
    }
}

#[pin_project::pin_project]
pub struct Future<S, T, U>(#[pin] State<S, T, U>)
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    T: Default,
    U: http_body::Body;

#[pin_project::pin_project(project = StateProj)]
#[allow(clippy::large_enum_variant)]
enum State<S, T, U>
where
    S: tower::Service<http::Request<T>>,
    U: http_body::Body,
{
    S0(Option<Builder<S, T>>),
    S1(#[pin] tower::util::Oneshot<S, http::Request<T>>),
    S2(
        #[pin] http_body_util::combinators::Collect<U>,
        Option<http::response::Parts>,
    ),
}

impl<S, T, U> future::Future for Future<S, T, U>
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    T: Default,
    U: http_body::Body,
{
    type Output = Result<http::Response<U>, Error<S::Error, U::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            match this.0.as_mut().project() {
                StateProj::S0(state) => {
                    let Builder { service, uri, .. } = state.take().unwrap();
                    let request = http::Request::get(uri)
                        .body(T::default())
                        .map_err(Error::Http)?;
                    this.0.set(State::S1(service.oneshot(request)));
                }
                StateProj::S1(f) => {
                    let response = ready!(f.poll(cx)).map_err(Error::Service)?;
                    if response.status().is_success() {
                        break Poll::Ready(Ok(response));
                    } else {
                        let (parts, body) = response.into_parts();
                        this.0.set(State::S2(body.collect(), Some(parts)));
                    }
                }
                StateProj::S2(f, state) => {
                    let body = ready!(f.poll(cx)).map_err(Error::Body)?;
                    let parts = state.take().unwrap();
                    break Poll::Ready(Err(Error::Api(http::Response::from_parts(
                        parts,
                        body.to_bytes(),
                    ))));
                }
            }
        }
    }
}
