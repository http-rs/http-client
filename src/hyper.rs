//! http-client implementation for reqwest
use super::{Error, HttpClient, Request, Response};
use http_types::StatusCode;
use hyper::body::HttpBody;
use hyper_tls::HttpsConnector;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

/// Hyper-based HTTP Client.
#[derive(Debug)]
struct HyperClient {}

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
    async fn try_from(mut value: http_types::Request) -> Result<Self, Error> {
        // Note: Much of this code was taken from the `http-types` compat implementation. Trying to
        // figure out the feature flags to conditionally compile with compat support was rather
        // difficult, so copying code was deemed a reasonable intermediate solution.
        // Also, because converting the `http_types` body to bytes is async, we can't implement `TryFrom`

        // TODO: Do this without a `String` allocation
        let method = hyper::Method::from_str(&value.method().to_string()).unwrap();

        let version = value
            .version()
            .map(|v| match v {
                http_types::Version::Http0_9 => Ok(hyper::Version::HTTP_09),
                http_types::Version::Http1_0 => Ok(hyper::Version::HTTP_10),
                http_types::Version::Http1_1 => Ok(hyper::Version::HTTP_11),
                http_types::Version::Http2_0 => Ok(hyper::Version::HTTP_2),
                http_types::Version::Http3_0 => Ok(hyper::Version::HTTP_3),
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
    inner: http_types::Response,
}

impl HttpTypesResponse {
    async fn try_from(value: hyper::Response<hyper::Body>) -> Result<Self, Error> {
        // Note: Much of this code was taken from the `http-types` compat implementation. Trying to
        // figure out the feature flags to conditionally compile with compat support was rather
        // difficult, so copying code was deemed a reasonable intermediate solution.
        let (parts, mut body) = value.into_parts();

        // UNWRAP: http and http-types implement the same status codes
        let status: StatusCode = parts.status.as_u16().try_into().unwrap();

        let version = match parts.version {
            hyper::Version::HTTP_09 => Ok(http_types::Version::Http0_9),
            hyper::Version::HTTP_10 => Ok(http_types::Version::Http1_0),
            hyper::Version::HTTP_11 => Ok(http_types::Version::Http1_1),
            hyper::Version::HTTP_2 => Ok(http_types::Version::Http2_0),
            hyper::Version::HTTP_3 => Ok(http_types::Version::Http3_0),
            // TODO: Is this realistically reachable, and should it be marked BadRequest?
            _ => Err(Error::from_str(
                StatusCode::BadRequest,
                "unrecognized HTTP response version",
            )),
        }?;

        let body = match body.data().await {
            None => None,
            Some(Ok(b)) => Some(b),
            Some(Err(_)) => {
                return Err(Error::from_str(
                    StatusCode::BadRequest,
                    "unable to read HTTP response body",
                ))
            }
        }
        .map(|b| http_types::Body::from_bytes(b.to_vec()))
        // TODO: How does `http-types` handle responses without bodies?
        .unwrap_or(http_types::Body::from_bytes(Vec::new()));

        let mut res = Response::new(status);
        res.set_version(Some(version));

        for (name, value) in parts.headers {
            // TODO: http_types uses an `unsafe` block here, should it be allowed for `hyper` as well?
            let value = value.as_bytes().to_owned();
            let value = http_types::headers::HeaderValue::from_bytes(value)?;

            if let Some(name) = name {
                let name = name.as_str();
                let name = http_types::headers::HeaderName::from_str(name)?;
                res.insert_header(name, value);
            }
        }

        res.set_body(body);
        Ok(HttpTypesResponse { inner: res })
    }

    fn into_inner(self) -> http_types::Response {
        self.inner
    }
}
