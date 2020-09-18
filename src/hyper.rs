//! http-client implementation for reqwest

use super::{async_trait, Error, HttpClient, Request, Response};
use futures_util::stream::TryStreamExt;
use http_types::headers::{HeaderName, HeaderValue};
use http_types::StatusCode;
use hyper::body::HttpBody;
use hyper_tls::HttpsConnector;
use std::convert::TryFrom;
use std::io;
use std::str::FromStr;

/// Hyper-based HTTP Client.
#[derive(Debug)]
pub struct HyperClient {}

impl HyperClient {
    /// Create a new client.
    ///
    /// There is no specific benefit to reusing instances of this client.
    pub fn new() -> Self {
        HyperClient {}
    }
}

#[async_trait]
impl HttpClient for HyperClient {
    async fn send(&self, req: Request) -> Result<Response, Error> {
        let req = HyperHttpRequest::try_from(req).await?.into_inner();
        // UNWRAP: Scheme guaranteed to be "http" or "https" as part of conversion
        let scheme = req.uri().scheme_str().unwrap();

        let response = match scheme {
            "http" => {
                let client = hyper::Client::builder().build_http::<hyper::Body>();
                client.request(req).await
            }
            "https" => {
                let https = HttpsConnector::new();
                let client = hyper::Client::builder().build::<_, hyper::Body>(https);
                client.request(req).await
            }
            _ => unreachable!(),
        }?;

        let resp = HttpTypesResponse::try_from(response).await?.into_inner();
        Ok(resp)
    }
}

struct HyperHttpRequest {
    inner: hyper::Request<hyper::Body>,
}

impl HyperHttpRequest {
    async fn try_from(mut value: Request) -> Result<Self, Error> {
        // UNWRAP: This unwrap is unjustified in `http-types`, need to check if it's actually safe.
        let uri = hyper::Uri::try_from(&format!("{}", value.url())).unwrap();

        // `HyperClient` depends on the scheme being either "http" or "https"
        match uri.scheme_str() {
            Some("http") | Some("https") => (),
            _ => return Err(Error::from_str(StatusCode::BadRequest, "invalid scheme")),
        };

        let mut request = hyper::Request::builder();

        // UNWRAP: Default builder is safe
        let req_headers = request.headers_mut().unwrap();
        for (name, values) in &value {
            // UNWRAP: http-types and http have equivalent validation rules
            let name = hyper::header::HeaderName::from_str(name.as_str()).unwrap();

            for value in values.iter() {
                // UNWRAP: http-types and http have equivalent validation rules
                let value =
                    hyper::header::HeaderValue::from_bytes(value.as_str().as_bytes()).unwrap();
                req_headers.append(&name, value);
            }
        }

        let body = value.body_bytes().await?;
        let body = hyper::Body::from(body);

        let request = request
            .method(value.method())
            .version(value.version().map(|v| v.into()).unwrap_or_default())
            .uri(uri)
            .body(body)?;

        Ok(HyperHttpRequest { inner: request })
    }

    fn into_inner(self) -> hyper::Request<hyper::Body> {
        self.inner
    }
}

struct HttpTypesResponse {
    inner: Response,
}

impl HttpTypesResponse {
    async fn try_from(value: hyper::Response<hyper::Body>) -> Result<Self, Error> {
        let (parts, body) = value.into_parts();

        let size_hint = body.size_hint().upper().map(|s| s as usize);
        let body = body.map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()));
        let body = http_types::Body::from_reader(body.into_async_read(), size_hint);

        let mut res = Response::new(parts.status);
        res.set_version(Some(parts.version.into()));

        for (name, value) in parts.headers {
            let value = value.as_bytes().to_owned();
            let value = HeaderValue::from_bytes(value)?;

            if let Some(name) = name {
                let name = name.as_str();
                let name = HeaderName::from_str(name)?;
                res.insert_header(name, value);
            }
        }

        res.set_body(body);
        Ok(HttpTypesResponse { inner: res })
    }

    fn into_inner(self) -> Response {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, HttpClient};
    use http_types::{Method, Request, Url};
    use hyper::service::{make_service_fn, service_fn};
    use std::time::Duration;
    use tokio::sync::oneshot::channel;

    use super::HyperClient;

    async fn echo(
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
        Ok(hyper::Response::new(req.into_body()))
    }

    #[tokio::test]
    async fn basic_functionality() {
        let (send, recv) = channel::<()>();

        let recv = async move { recv.await.unwrap_or(()) };

        let addr = ([127, 0, 0, 1], portpicker::pick_unused_port().unwrap()).into();
        let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(echo)) });
        let server = hyper::Server::bind(&addr)
            .serve(service)
            .with_graceful_shutdown(recv);

        let client = HyperClient::new();
        let url = Url::parse(&format!("http://localhost:{}", addr.port())).unwrap();
        let mut req = Request::new(Method::Get, url);
        req.set_body("hello");

        let client = async move {
            tokio::time::delay_for(Duration::from_millis(100)).await;
            let mut resp = client.send(req).await?;
            send.send(()).unwrap();
            assert_eq!(resp.body_string().await?, "hello");

            Result::<(), Error>::Ok(())
        };

        let (client_res, server_res) = tokio::join!(client, server);
        assert!(client_res.is_ok());
        assert!(server_res.is_ok());
    }
}
