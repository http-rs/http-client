//! http-client implementation for reqwest

use super::{Error, HttpClient, Request, Response};
use http_types::headers::{HeaderName, HeaderValue};
use http_types::{Method, StatusCode, Version};
use hyper::body::HttpBody;
use hyper_tls::HttpsConnector;
use std::convert::{TryFrom, TryInto};
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

impl HttpClient for HyperClient {
    fn send(&self, req: Request) -> futures::future::BoxFuture<'static, Result<Response, Error>> {
        Box::pin(async move {
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
        })
    }
}

struct HyperHttpRequest {
    inner: hyper::Request<hyper::Body>,
}

impl HyperHttpRequest {
    async fn try_from(mut value: Request) -> Result<Self, Error> {
        let method = match value.method() {
            Method::Get => hyper::Method::GET,
            Method::Head => hyper::Method::HEAD,
            Method::Post => hyper::Method::POST,
            Method::Put => hyper::Method::PUT,
            Method::Patch => hyper::Method::PATCH,
            Method::Options => hyper::Method::OPTIONS,
            Method::Trace => hyper::Method::TRACE,
            Method::Connect => hyper::Method::CONNECT,
            _ => {
                return Err(Error::from_str(
                    StatusCode::BadRequest,
                    "unrecognized HTTP method",
                ))
            }
        };

        let version = value
            .version()
            .map(|v| match v {
                Version::Http0_9 => Ok(hyper::Version::HTTP_09),
                Version::Http1_0 => Ok(hyper::Version::HTTP_10),
                Version::Http1_1 => Ok(hyper::Version::HTTP_11),
                Version::Http2_0 => Ok(hyper::Version::HTTP_2),
                Version::Http3_0 => Ok(hyper::Version::HTTP_3),
                _ => Err(Error::from_str(
                    StatusCode::BadRequest,
                    "unrecognized HTTP version",
                )),
            })
            .or(Some(Ok(hyper::Version::default())))
            .unwrap()?;

        // UNWRAP: This unwrap is unjustified in `http-types`, need to check if it's actually safe.
        let uri = hyper::Uri::try_from(&format!("{}", value.url())).unwrap();

        // `HttpClient` depends on the scheme being either "http" or "https"
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

        let req = hyper::Request::builder()
            .method(method)
            .version(version)
            .uri(uri)
            .body(body)?;

        Ok(HyperHttpRequest { inner: req })
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
        let (parts, mut body) = value.into_parts();

        // UNWRAP: http and http-types implement the same status codes
        let status: StatusCode = parts.status.as_u16().try_into().unwrap();

        let version = match parts.version {
            hyper::Version::HTTP_09 => Ok(http_types::Version::Http0_9),
            hyper::Version::HTTP_10 => Ok(http_types::Version::Http1_0),
            hyper::Version::HTTP_11 => Ok(http_types::Version::Http1_1),
            hyper::Version::HTTP_2 => Ok(http_types::Version::Http2_0),
            hyper::Version::HTTP_3 => Ok(http_types::Version::Http3_0),
            _ => Err(Error::from_str(
                StatusCode::BadGateway,
                "unrecognized HTTP response version",
            )),
        }?;

        let body = match body.data().await {
            None => None,
            Some(Ok(b)) => Some(b),
            Some(Err(_)) => {
                return Err(Error::from_str(
                    StatusCode::BadGateway,
                    "unable to read HTTP response body",
                ))
            }
        }
        .map(|b| http_types::Body::from_bytes(b.to_vec()))
        .unwrap_or(http_types::Body::empty());

        let mut res = Response::new(status);
        res.set_version(Some(version));

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
