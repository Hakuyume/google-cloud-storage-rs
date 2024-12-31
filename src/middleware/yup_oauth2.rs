use futures::future::BoxFuture;
use futures::FutureExt;
use headers::{Authorization, HeaderMapExt};
use hyper_util::client::legacy::connect::Connect;
use std::env;
use std::future;
use std::mem;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use yup_oauth2::authenticator::{
    ApplicationDefaultCredentialsTypes, Authenticator, HyperClientBuilder,
};

#[derive(Debug, thiserror::Error)]
pub enum Error<S> {
    #[error(transparent)]
    Authenticator(yup_oauth2::Error),
    #[error(transparent)]
    InvalidBearerToken(headers::authorization::InvalidBearerToken),
    #[error(transparent)]
    Service(S),
}

pub async fn with_client<C>(client: C) -> Result<Layer<C::Connector>, yup_oauth2::Error>
where
    C: HyperClientBuilder,
{
    let authenticator = async {
        if let Ok(path) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            if let Ok(secret) = yup_oauth2::read_authorized_user_secret(&path).await {
                return yup_oauth2::AuthorizedUserAuthenticator::with_client(secret, client)
                    .build()
                    .await;
            } else if let Ok(secret) = yup_oauth2::read_external_account_secret(&path).await {
                return yup_oauth2::ExternalAccountAuthenticator::with_client(secret, client)
                    .build()
                    .await;
            }
        }
        match yup_oauth2::ApplicationDefaultCredentialsAuthenticator::with_client(
            yup_oauth2::ApplicationDefaultCredentialsFlowOpts::default(),
            client,
        )
        .await
        {
            ApplicationDefaultCredentialsTypes::ServiceAccount(builder) => builder.build().await,
            ApplicationDefaultCredentialsTypes::InstanceMetadata(builder) => builder.build().await,
        }
    }
    .await?;

    Ok(Layer { authenticator })
}

#[derive(Clone)]
pub struct Layer<C> {
    authenticator: Authenticator<C>,
}

impl<S, C> tower::Layer<S> for Layer<C>
where
    C: Clone,
{
    type Service = Service<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        Service {
            inner,
            authenticator: self.authenticator.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Service<S, C> {
    inner: S,
    authenticator: Authenticator<C>,
}

impl<S, T, C> tower::Service<http::Request<T>> for Service<S, C>
where
    S: Clone + tower::Service<http::Request<T>>,
    C: Clone + Connect + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = Error<S::Error>;
    type Future = Future<S, T, C>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Error::Service)
    }

    fn call(&mut self, request: http::Request<T>) -> Self::Future {
        let inner = self.inner.clone();
        let inner = mem::replace(&mut self.inner, inner);
        Future(
            inner,
            State::S0(Some((self.authenticator.clone(), request))),
        )
    }
}

#[pin_project::pin_project]
pub struct Future<S, T, C>(S, #[pin] State<S, T, C>)
where
    S: tower::Service<http::Request<T>>;

#[pin_project::pin_project(project = StateProj)]
enum State<S, T, C>
where
    S: tower::Service<http::Request<T>>,
{
    S0(Option<(Authenticator<C>, http::Request<T>)>),
    S1(
        #[pin] BoxFuture<'static, Result<yup_oauth2::AccessToken, yup_oauth2::Error>>,
        Option<http::Request<T>>,
    ),
    S2(#[pin] S::Future),
}

impl<S, T, C> future::Future for Future<S, T, C>
where
    S: tower::Service<http::Request<T>>,
    C: Clone + Connect + Send + Sync + 'static,
{
    type Output = Result<S::Response, Error<S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            match this.1.as_mut().project() {
                StateProj::S0(state) => {
                    let (authenticator, request) = state.take().unwrap();
                    let f = async move {
                        authenticator
                            .token(&["https://www.googleapis.com/auth/cloud-platform"])
                            .await
                    };
                    this.1.set(State::S1(f.boxed(), Some(request)));
                }
                StateProj::S1(f, state) => {
                    let access_token = ready!(f.poll(cx)).map_err(Error::Authenticator)?;
                    let mut request = state.take().unwrap();
                    let token = access_token
                        .token()
                        .ok_or(Error::Authenticator(yup_oauth2::Error::MissingAccessToken))?;
                    let authorization =
                        Authorization::bearer(token).map_err(Error::InvalidBearerToken)?;
                    request.headers_mut().typed_insert(authorization);
                    this.1.set(State::S2(this.0.call(request)));
                }
                StateProj::S2(f) => {
                    let response = ready!(f.poll(cx)).map_err(Error::Service)?;
                    break Poll::Ready(Ok(response));
                }
            }
        }
    }
}
