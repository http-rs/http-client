//! http-client implementation for async-h1.

use super::{HttpClient, Request, Response};

use async_h1::client;
use futures::future::BoxFuture;
use http_types::{Error, StatusCode};

/// Async-h1 based HTTP Client.
#[derive(Debug)]
pub struct H1Client {}

impl Default for H1Client {
    fn default() -> Self {
        Self::new()
    }
}

impl H1Client {
    /// Create a new instance.
    pub fn new() -> Self {
        Self {}
    }
}

impl Clone for H1Client {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl HttpClient for H1Client {
    type Error = Error;

    fn send(&self, req: Request) -> BoxFuture<'static, Result<Response, Self::Error>> {
        Box::pin(async move {
            // Insert host
            let host = req
                .url()
                .host_str()
                .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing hostname"))?;

            let scheme = req.url().scheme();
            if scheme != "http" && scheme != "https" {
                return Err(Error::from_str(
                    StatusCode::BadRequest,
                    format!("invalid url scheme '{}'", scheme),
                ));
            }

            let addr = req
                .url()
                .socket_addrs(|| match req.url().scheme() {
                    "http" => Some(80),
                    "https" => Some(443),
                    _ => None,
                })?
                .into_iter()
                .next()
                .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing valid address"))?;

            log::trace!("> Scheme: {}", scheme);

            match scheme {
                "http" => {
                    let stream = async_std::net::TcpStream::connect(addr).await?;
                    client::connect(stream, req).await
                }
                "https" => {
                    let raw_stream = async_std::net::TcpStream::connect(addr).await?;

                    let stream = async_native_tls::connect(host, raw_stream).await?;

                    client::connect(stream, req).await
                }
                _ => unreachable!(),
            }
        })
    }
}
