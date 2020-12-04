//! http-client implementation for async-h1.

use super::{async_trait, Error, HttpClient, Request, Response};

use async_h1::client;
use http_types::StatusCode;

/// Async-h1 based HTTP Client.
#[derive(Debug)]
pub struct H1Client {
    _priv: (),
}

impl Default for H1Client {
    fn default() -> Self {
        Self::new()
    }
}

impl H1Client {
    /// Create a new instance.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

#[async_trait]
impl HttpClient for H1Client {
    async fn send(&self, mut req: Request) -> Result<Response, Error> {
        // Insert host
        let host = req
            .url()
            .host_str()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing hostname"))?
            .to_string();

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
                req.set_peer_addr(stream.peer_addr().ok());
                req.set_local_addr(stream.local_addr().ok());
                client::connect(stream, req).await
            }
            "https" => {
                let raw_stream = async_std::net::TcpStream::connect(addr).await?;
                req.set_peer_addr(raw_stream.peer_addr().ok());
                req.set_local_addr(raw_stream.local_addr().ok());
                let tls_stream = add_tls(host, raw_stream).await?;
                client::connect(tls_stream, req).await
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(not(feature = "h1_client_rustls"))]
async fn add_tls(
    host: String,
    stream: async_std::net::TcpStream,
) -> Result<async_native_tls::TlsStream<async_std::net::TcpStream>, async_native_tls::Error> {
    async_native_tls::connect(host, stream).await
}

#[cfg(feature = "h1_client_rustls")]
async fn add_tls(
    host: String,
    stream: async_std::net::TcpStream,
) -> std::io::Result<async_tls::client::TlsStream<async_std::net::TcpStream>> {
    let connector = async_tls::TlsConnector::default();
    connector.connect(host, stream).await
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
            let mut response: Response = H1Client::new().send(request).await?;
            assert_eq!(response.body_string().await.unwrap(), "hello");
            Ok(())
        });

        server.race(client).await?;

        Ok(())
    }

    #[async_std::test]
    async fn https_functionality() -> Result<()> {
        task::sleep(Duration::from_millis(100)).await;
        // Send a POST request to https://httpbin.org/post
        // The result should be a JSon string similar to what you get with:
        //  curl -X POST "https://httpbin.org/post" -H "accept: application/json" -H "Content-Type: text/plain;charset=utf-8" -d "hello"
        let request = build_test_request(Url::parse("https://httpbin.org/post").unwrap());
        let mut response: Response = H1Client::new().send(request).await?;
        let json_val: serde_json::value::Value =
            serde_json::from_str(&response.body_string().await.unwrap())?;
        assert_eq!(*json_val.get("data").unwrap(), serde_json::json!("hello"));
        Ok(())
    }
}
