use futures::future::BoxFuture;
use futures::FutureExt;
use headers::{Authorization, HeaderMapExt};
use http::Request;
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

#[derive(Clone)]
pub struct Service<S, C> {
    inner: S,
    authenticator: Authenticator<C>,
}

impl<S, C, B> tower::Service<Request<B>> for Service<S, C>
where
    S: Clone + tower::Service<Request<B>>,
    C: Clone + Connect + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = Error<S::Error>;
    type Future = Future<S, B>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Error::Service)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let inner = self.inner.clone();
        let inner = mem::replace(&mut self.inner, inner);
        let authenticator = self.authenticator.clone();
        let f = async move {
            authenticator
                .token(&["https://www.googleapis.com/auth/cloud-platform"])
                .await
        }
        .boxed();
        Future(State::S0 {
            f,
            inner,
            request: Some(request),
        })
    }
}

#[pin_project::pin_project]
pub struct Future<S, B>(#[pin] State<S, B>)
where
    S: tower::Service<Request<B>>;

#[pin_project::pin_project(project = StateProj)]
enum State<S, B>
where
    S: tower::Service<Request<B>>,
{
    S0 {
        #[pin]
        f: BoxFuture<'static, Result<yup_oauth2::AccessToken, yup_oauth2::Error>>,
        inner: S,
        request: Option<Request<B>>,
    },
    S1 {
        #[pin]
        f: S::Future,
    },
}

impl<S, B> future::Future for Future<S, B>
where
    S: tower::Service<Request<B>>,
{
    type Output = Result<S::Response, Error<S::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            match this.0.as_mut().project() {
                StateProj::S0 { f, inner, request } => {
                    let access_token = ready!(f.poll(cx)).map_err(Error::Authenticator)?;
                    let token = access_token
                        .token()
                        .ok_or(Error::Authenticator(yup_oauth2::Error::MissingAccessToken))?;
                    let header = Authorization::bearer(token).map_err(Error::InvalidBearerToken)?;
                    let mut request = request.take().unwrap();
                    request.headers_mut().typed_insert(header);
                    let f = inner.call(request);
                    this.0.set(State::S1 { f });
                }
                StateProj::S1 { f } => {
                    let response = ready!(f.poll(cx)).map_err(Error::Service)?;
                    break Poll::Ready(Ok(response));
                }
            }
        }
    }
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

impl<C> Layer<C>
where
    C: Clone + Connect + Send + Sync + 'static,
{
    pub async fn with_client<Client>(client: Client) -> Result<Self, yup_oauth2::Error>
    where
        Client: HyperClientBuilder<Connector = C>,
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
                ApplicationDefaultCredentialsTypes::ServiceAccount(builder) => {
                    builder.build().await
                }
                ApplicationDefaultCredentialsTypes::InstanceMetadata(builder) => {
                    builder.build().await
                }
            }
        }
        .await?;
        Ok(Layer { authenticator })
    }
}
