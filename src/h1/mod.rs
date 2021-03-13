//! http-client implementation for async-h1, with connecton pooling ("Keep-Alive").

use std::fmt::Debug;
use std::net::SocketAddr;

use async_h1::client;
use async_std::net::TcpStream;
use dashmap::DashMap;
use deadpool::managed::Pool;
use http_types::StatusCode;

cfg_if::cfg_if! {
    if #[cfg(feature = "rustls")] {
        use async_tls::client::TlsStream;
    } else if #[cfg(feature = "native-tls")] {
        use async_native_tls::TlsStream;
    }
}

use super::{async_trait, Error, HttpClient, Request, Response};

// #[cfg(feature = "unstable-config")]
use super::config::Config;

mod tcp;
#[cfg(any(feature = "native-tls", feature = "rustls"))]
mod tls;

use tcp::{TcpConnWrapper, TcpConnection};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use tls::{TlsConnWrapper, TlsConnection};

// This number is based on a few random benchmarks and see whatever gave decent perf vs resource use.
const DEFAULT_MAX_CONCURRENT_CONNECTIONS: usize = 50;

type HttpPool = DashMap<SocketAddr, Pool<TcpStream, std::io::Error>>;
#[cfg(any(feature = "native-tls", feature = "rustls"))]
type HttpsPool = DashMap<SocketAddr, Pool<TlsStream<TcpStream>, Error>>;

/// Async-h1 based HTTP Client, with connecton pooling ("Keep-Alive").
pub struct H1Client {
    http_pools: HttpPool,
    #[cfg(any(feature = "native-tls", feature = "rustls"))]
    https_pools: HttpsPool,
    max_concurrent_connections: usize,
    // #[cfg(feature = "unstable-config")]
    config: Option<Config>,
}

impl Debug for H1Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let https_pools = if cfg!(any(feature = "native-tls", feature = "rustls")) {
            self.http_pools
                .iter()
                .map(|pool| {
                    let status = pool.status();
                    format!(
                        "Connections: {}, Available: {}, Max: {}",
                        status.size, status.available, status.max_size
                    )
                })
                .collect::<Vec<String>>()
        } else {
            vec![]
        };

        f.debug_struct("H1Client")
            .field(
                "http_pools",
                &self
                    .http_pools
                    .iter()
                    .map(|pool| {
                        let status = pool.status();
                        format!(
                            "Connections: {}, Available: {}, Max: {}",
                            status.size, status.available, status.max_size
                        )
                    })
                    .collect::<Vec<String>>(),
            )
            .field("https_pools", &https_pools)
            .field(
                "max_concurrent_connections",
                &self.max_concurrent_connections,
            )
            .finish()
    }
}

impl Default for H1Client {
    fn default() -> Self {
        Self::new()
    }
}

impl H1Client {
    /// Create a new instance.
    pub fn new() -> Self {
        Self {
            http_pools: DashMap::new(),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            https_pools: DashMap::new(),
            max_concurrent_connections: DEFAULT_MAX_CONCURRENT_CONNECTIONS,
            // #[cfg(feature = "unstable-config")]
            config: None,
        }
    }

    /// Create a new instance.
    pub fn with_max_connections(max: usize) -> Self {
        Self {
            http_pools: DashMap::new(),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            https_pools: DashMap::new(),
            max_concurrent_connections: max,
            // #[cfg(feature = "unstable-config")]
            config: None,
        }
    }

    // #[cfg(feature = "unstable-config")]
    /// Override the existing configuration with new configuration.
    pub fn override_config(&mut self, config: Config) {
        self.config = Some(config);
    }

    /// Update the existing configuration with new configuration.
    pub fn update_config(&mut self, _config: Config) {
        todo!("Only update orperties which ahve been changed on the config object.")
    }

    /// Override the existing configuration with new configuration.
    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }
}

#[async_trait]
impl HttpClient for H1Client {
    async fn send(&self, mut req: Request) -> Result<Response, Error> {
        req.insert_header("Connection", "keep-alive");

        // Insert host
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        let host = req
            .url()
            .host_str()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing hostname"))?
            .to_string();

        let scheme = req.url().scheme();
        if scheme != "http"
            && (scheme != "https" || cfg!(not(any(feature = "native-tls", feature = "rustls"))))
        {
            return Err(Error::from_str(
                StatusCode::BadRequest,
                format!("invalid url scheme '{}'", scheme),
            ));
        }

        let addrs = req.url().socket_addrs(|| match req.url().scheme() {
            "http" => Some(80),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            "https" => Some(443),
            _ => None,
        })?;

        log::trace!("> Scheme: {}", scheme);

        let max_addrs_idx = addrs.len() - 1;
        for (idx, addr) in addrs.into_iter().enumerate() {
            let has_another_addr = idx != max_addrs_idx;

            match scheme {
                "http" => {
                    let pool_ref = if let Some(pool_ref) = self.http_pools.get(&addr) {
                        pool_ref
                    } else {
                        let manager = TcpConnection::new(addr);
                        let pool = Pool::<TcpStream, std::io::Error>::new(
                            manager,
                            self.max_concurrent_connections,
                        );
                        self.http_pools.insert(addr, pool);
                        self.http_pools.get(&addr).unwrap()
                    };

                    // Deadlocks are prevented by cloning an inner pool Arc and dropping the original locking reference before we await.
                    let pool = pool_ref.clone();
                    std::mem::drop(pool_ref);

                    let stream = match pool.get().await {
                        Ok(s) => s,
                        Err(_) if has_another_addr => continue,
                        Err(e) => return Err(Error::from_str(400, e.to_string()))?,
                    };

                    req.set_peer_addr(stream.peer_addr().ok());
                    req.set_local_addr(stream.local_addr().ok());
                    return client::connect(TcpConnWrapper::new(stream), req).await;
                }
                #[cfg(any(feature = "native-tls", feature = "rustls"))]
                "https" => {
                    let pool_ref = if let Some(pool_ref) = self.https_pools.get(&addr) {
                        pool_ref
                    } else {
                        let manager = TlsConnection::new(host.clone(), addr);
                        let pool = Pool::<TlsStream<TcpStream>, Error>::new(
                            manager,
                            self.max_concurrent_connections,
                        );
                        self.https_pools.insert(addr, pool);
                        self.https_pools.get(&addr).unwrap()
                    };

                    // Deadlocks are prevented by cloning an inner pool Arc and dropping the original locking reference before we await.
                    let pool = pool_ref.clone();
                    std::mem::drop(pool_ref);

                    let stream = match pool.get().await {
                        Ok(s) => s,
                        Err(_) if has_another_addr => continue,
                        Err(e) => return Err(Error::from_str(400, e.to_string()))?,
                    };

                    req.set_peer_addr(stream.get_ref().peer_addr().ok());
                    req.set_local_addr(stream.get_ref().local_addr().ok());

                    return client::connect(TlsConnWrapper::new(stream), req).await;
                }
                _ => unreachable!(),
            }
        }

        Err(Error::from_str(
            StatusCode::BadRequest,
            "missing valid address",
        ))
    }
}

// #[cfg(feature = "unstable-config")]
impl<C: Into<Config>> From<C> for H1Client {
    fn from(config: C) -> Self {
        let config = config.into();

        Self {
            http_pools: DashMap::new(),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            https_pools: DashMap::new(),
            max_concurrent_connections: DEFAULT_MAX_CONCURRENT_CONNECTIONS,
            config: Some(config),
        }
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
