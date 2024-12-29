use super::Error;
use bytes::Bytes;
use http_body_util::BodyExt;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tower::ServiceExt;

pub(super) fn oneshot<S, T, U>(
    service: S,
    request: Result<http::Request<T>, Error<S::Error, U::Error>>,
) -> Oneshot<S, T, U>
where
    S: tower::Service<http::Request<T>>,
    U: http_body::Body,
{
    match request {
        Ok(request) => Oneshot::S0(service.oneshot(request)),
        Err(e) => Oneshot::S1(Some(e)),
    }
}
#[pin_project::pin_project(project = OneshotProj)]
#[allow(clippy::large_enum_variant)]
pub(super) enum Oneshot<S, T, U>
where
    S: tower::Service<http::Request<T>>,
    U: http_body::Body,
{
    S0(#[pin] tower::util::Oneshot<S, http::Request<T>>),
    S1(Option<Error<S::Error, U::Error>>),
    S2(
        #[pin] http_body_util::combinators::Collect<U>,
        Option<http::response::Parts>,
    ),
}
impl<S, T, U> Future for Oneshot<S, T, U>
where
    S: tower::Service<http::Request<T>, Response = http::Response<U>>,
    U: http_body::Body,
{
    type Output = Result<http::Response<U>, Error<S::Error, U::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                OneshotProj::S0(f) => {
                    let response = ready!(f.poll(cx)).map_err(Error::Service)?;
                    if response.status().is_success() {
                        break Poll::Ready(Ok(response));
                    } else {
                        let (parts, body) = response.into_parts();
                        self.set(Self::S2(body.collect(), Some(parts)));
                    }
                }
                OneshotProj::S1(state) => {
                    let e = state.take().unwrap();
                    break Poll::Ready(Err(e));
                }
                OneshotProj::S2(f, state) => {
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

pub(super) fn collect<F, U>(f: F) -> Collect<F, U>
where
    U: http_body::Body,
{
    Collect::S0(f)
}
#[pin_project::pin_project(project = CollectProj)]
#[allow(clippy::large_enum_variant)]
pub(super) enum Collect<F, U>
where
    U: http_body::Body,
{
    S0(#[pin] F),
    S1(
        #[pin] http_body_util::combinators::Collect<U>,
        Option<http::response::Parts>,
    ),
}
impl<F, U, SE> Future for Collect<F, U>
where
    F: Future<Output = Result<http::Response<U>, Error<SE, U::Error>>>,
    U: http_body::Body,
{
    type Output = Result<http::Response<Bytes>, Error<SE, U::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                CollectProj::S0(f) => {
                    let response = ready!(f.poll(cx))?;
                    let (parts, body) = response.into_parts();
                    self.set(Self::S1(body.collect(), Some(parts)));
                }
                CollectProj::S1(f, state) => {
                    let body = ready!(f.poll(cx)).map_err(Error::Body)?;
                    let parts = state.take().unwrap();
                    break Poll::Ready(Ok(http::Response::from_parts(parts, body.to_bytes())));
                }
            }
        }
    }
}
