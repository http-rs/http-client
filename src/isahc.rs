//! http-client implementation for isahc

use super::{Body, HttpClient, Request, Response};

use futures::future::BoxFuture;

use std::sync::Arc;

/// Curl-based HTTP Client.
#[derive(Debug)]
pub struct IsahcClient {
    client: Arc<isahc::HttpClient>,
}

impl Default for IsahcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl IsahcClient {
    /// Create a new instance.
    pub fn new() -> Self {
        Self::from_client(isahc::HttpClient::new().unwrap())
    }

    /// Create from externally initialized and configured client.
    pub fn from_client(client: isahc::HttpClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl Clone for IsahcClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl HttpClient for IsahcClient {
    type Error = isahc::Error;

    fn send(&self, req: Request) -> BoxFuture<'static, Result<Response, Self::Error>> {
        let client = self.client.clone();
        Box::pin(async move {
            let (parts, body) = req.into_parts();

            let body = if body.is_empty() {
                isahc::Body::empty()
            } else {
                match body.len {
                    Some(len) => isahc::Body::from_reader_sized(body, len),
                    None => isahc::Body::from_reader(body),
                }
            };

            let req: http::Request<isahc::Body> = http::Request::from_parts(parts, body);

            let res = client.send_async(req).await?;

            let (parts, body) = res.into_parts();
            let body = Body::from_reader(body);
            let res = http::Response::from_parts(parts, body);
            Ok(res)
        })
    }
}
