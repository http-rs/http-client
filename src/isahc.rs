//! http-client implementation for isahc

use super::{async_trait, Body, Error, HttpClient, Request, Response};

use async_std::io::BufReader;
use isahc::{http, ResponseExt};

/// Curl-based HTTP Client.
#[derive(Debug)]
pub struct IsahcClient(isahc::HttpClient);

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
        Self(client)
    }
}

#[async_trait]
impl HttpClient for IsahcClient {
    async fn send(&self, mut req: Request) -> Result<Response, Error> {
        let mut builder = http::Request::builder()
            .uri(req.url().as_str())
            .method(http::Method::from_bytes(req.method().to_string().as_bytes()).unwrap());

        for (name, value) in req.iter() {
            builder = builder.header(name.as_str(), value.as_str());
        }

        let body = req.take_body();
        let body = match body.len() {
            Some(len) => isahc::AsyncBody::from_reader_sized(body, len as u64),
            None => isahc::AsyncBody::from_reader(body),
        };

        let request = builder.body(body).unwrap();
        let res = self.0.send_async(request).await.map_err(Error::from)?;
        let maybe_metrics = res.metrics().cloned();
        let (parts, body) = res.into_parts();
        let body = Body::from_reader(BufReader::new(body), None);
        let mut response = http_types::Response::new(parts.status.as_u16());
        for (name, value) in &parts.headers {
            response.insert_header(name.as_str(), value.to_str().unwrap());
        }

        if let Some(metrics) = maybe_metrics {
            response.ext_mut().insert(metrics);
        }

        response.set_body(body);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::prelude::*;
    use async_std::task;
    use http_types::url::Url;
    use http_types::Result;
    use std::time::Duration;

    fn build_test_request(url: Url) -> Request {
        let mut req = Request::new(http_types::Method::Post, url);
        req.set_body("hello");
        req.append_header("test", "value");
        req
    }

    #[async_std::test]
    async fn basic_functionality() -> Result<()> {
        let port = portpicker::pick_unused_port().unwrap();
        let mut app = tide::new();
        app.at("/").all(|mut r: tide::Request<()>| async move {
            let mut response = tide::Response::new(http_types::StatusCode::Ok);
            response.set_body(r.body_bytes().await.unwrap());
            Ok(response)
        });

        let server = task::spawn(async move {
            app.listen(("localhost", port)).await?;
            Result::Ok(())
        });

        let client = task::spawn(async move {
            task::sleep(Duration::from_millis(100)).await;
            let request =
                build_test_request(Url::parse(&format!("http://localhost:{}/", port)).unwrap());
            let mut response: Response = IsahcClient::new().send(request).await?;
            assert_eq!(response.body_string().await.unwrap(), "hello");
            Ok(())
        });

        server.race(client).await?;

        Ok(())
    }
}
