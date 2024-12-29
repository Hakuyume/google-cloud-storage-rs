use bytes::Bytes;
use headers::{ContentLength, ContentType, HeaderMapExt};
use http::StatusCode;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full};
use hyper_rustls::ConfigBuilderExt;
use md5::{Digest, Md5};
use std::convert::Infallible;
use std::env;
use std::sync::Arc;
use tower::Layer;

type Connector = hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
type Client = hyper_util::client::legacy::Client<Connector, UnsyncBoxBody<Bytes, Infallible>>;
type Service = crate::middleware::yup_oauth2::Service<Client, Connector>;
async fn service() -> Service {
    let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_native_roots()
    .unwrap()
    .with_no_client_auth();
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls_config)
        .https_only()
        .enable_http1()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector.clone());
    crate::middleware::yup_oauth2::with_connector(connector.clone())
        .await
        .unwrap()
        .layer(client)
}

fn bucket_name() -> String {
    env::var("BUCKET_NAME").unwrap()
}

fn object_name() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn body(data: &'static [u8]) -> UnsyncBoxBody<Bytes, Infallible> {
    Full::from(Bytes::from_static(data)).boxed_unsync()
}

fn assert_status<S, B>(e: super::Error<S, B>, status: StatusCode) {
    if let super::Error::Api(e) = e {
        assert_eq!(e.status(), status);
    } else {
        panic!();
    }
}

#[tokio::test]
async fn test_xml_head_object_no_such_key() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let e = super::xml::head_object::builder(&bucket_name, &object_name)
        .send(service)
        .await
        .unwrap_err();
    assert_status(e, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_xml_get_object_no_such_key() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let e = super::xml::get_object::builder(&bucket_name, &object_name)
        .send(service)
        .await
        .unwrap_err();
    assert_status(e, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_xml_delete_object_no_such_key() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let e = super::xml::delete_object::builder(&bucket_name, &object_name)
        .send(service)
        .await
        .unwrap_err();
    assert_status(e, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_xml_put_object() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let data = b"hello world";

    {
        let response = super::xml::put_object::builder(&bucket_name, &object_name, body(data))
            .send(service.clone())
            .await
            .unwrap();
        assert_eq!(
            response
                .headers()
                .typed_get::<crate::header::XGoogHash>()
                .unwrap()
                .md5,
            Some(Md5::digest(data).into()),
        );
    }
    {
        let response = super::xml::head_object::builder(&bucket_name, &object_name)
            .send(service.clone())
            .await
            .unwrap();
        assert_eq!(
            response.headers().typed_get::<ContentLength>().unwrap().0,
            data.len() as u64,
        );
        assert_eq!(
            response
                .headers()
                .typed_get::<crate::header::XGoogHash>()
                .unwrap()
                .md5,
            Some(Md5::digest(data).into()),
        );
    }
    {
        let response = super::xml::get_object::builder(&bucket_name, &object_name)
            .send(service.clone())
            .await
            .unwrap();
        assert_eq!(
            response.headers().typed_get::<ContentLength>().unwrap().0,
            data.len() as u64,
        );
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            data.as_slice(),
        );
    }
}

#[tokio::test]
async fn test_xml_put_object_content_type() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let data = b"hello world";

    {
        super::xml::put_object::builder(&bucket_name, &object_name, body(data))
            .content_type(mime::TEXT_PLAIN_UTF_8)
            .send(service.clone())
            .await
            .unwrap();
    }
    {
        let response = super::xml::head_object::builder(&bucket_name, &object_name)
            .send(service.clone())
            .await
            .unwrap();
        assert_eq!(
            mime::Mime::from(response.headers().typed_get::<ContentType>().unwrap()),
            mime::TEXT_PLAIN_UTF_8,
        );
    }
    {
        let response = super::xml::get_object::builder(&bucket_name, &object_name)
            .send(service.clone())
            .await
            .unwrap();
        assert_eq!(
            mime::Mime::from(response.headers().typed_get::<ContentType>().unwrap()),
            mime::TEXT_PLAIN_UTF_8,
        );
    }
}

#[tokio::test]
async fn test_xml_put_object_bad_digest() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let data = b"hello";
    let e = super::xml::put_object::builder(&bucket_name, &object_name, body(data))
        .content_md5(Md5::digest("world").into())
        .send(service)
        .await
        .unwrap_err();
    assert_status(e, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_xml_delete_object() {
    let service = service().await;
    let bucket_name = bucket_name();
    let object_name = object_name();
    let data = b"hello world";

    {
        super::xml::put_object::builder(&bucket_name, &object_name, body(data))
            .send(service.clone())
            .await
            .unwrap();
    }
    {
        super::xml::delete_object::builder(&bucket_name, &object_name)
            .send(service.clone())
            .await
            .unwrap();
    }
    {
        let e = super::xml::head_object::builder(&bucket_name, &object_name)
            .send(service)
            .await
            .unwrap_err();
        assert_status(e, StatusCode::NOT_FOUND);
    }
}
