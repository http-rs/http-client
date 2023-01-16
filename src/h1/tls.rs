use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use async_std::net::TcpStream;
use async_trait::async_trait;
use deadpool::managed::{Manager, Object, RecycleResult};
use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{Context, Poll};

cfg_if::cfg_if! {
    if #[cfg(feature = "h1-rustls")] {
        use std::convert::TryInto;
        use std::io;

        use async_rustls::client::TlsStream;
    } else if #[cfg(feature = "h1-native-tls")] {
        use async_native_tls::TlsStream;
    }
}

use crate::{Config, Error};

#[derive(Clone)]
#[cfg_attr(not(feature = "h1-rustls"), derive(std::fmt::Debug))]
pub(crate) struct TlsConnection {
    host: String,
    addr: SocketAddr,
    config: Arc<Config>,
}

impl TlsConnection {
    pub(crate) fn new(host: String, addr: SocketAddr, config: Arc<Config>) -> Self {
        Self { host, addr, config }
    }
}

pub(crate) struct TlsConnWrapper {
    conn: Object<TlsStream<TcpStream>, Error>,
}
impl TlsConnWrapper {
    pub(crate) fn new(conn: Object<TlsStream<TcpStream>, Error>) -> Self {
        Self { conn }
    }
}

impl AsyncRead for TlsConnWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut *self.conn).poll_read(cx, buf)
    }
}

impl AsyncWrite for TlsConnWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut *self.conn).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_close(cx)
    }
}

#[async_trait]
impl Manager<TlsStream<TcpStream>, Error> for TlsConnection {
    async fn create(&self) -> Result<TlsStream<TcpStream>, Error> {
        let raw_stream = async_std::net::TcpStream::connect(self.addr).await?;

        raw_stream.set_nodelay(self.config.tcp_no_delay)?;

        let tls_stream = add_tls(&self.host, raw_stream, &self.config).await?;
        Ok(tls_stream)
    }

    async fn recycle(&self, conn: &mut TlsStream<TcpStream>) -> RecycleResult<Error> {
        let mut buf = [0; 4];
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());

        #[cfg(feature = "h1-rustls")]
        conn.get_ref().0
            .set_nodelay(self.config.tcp_no_delay)
            .map_err(Error::from)?;
        #[cfg(feature = "h1-native-tls")]
        conn.get_ref()
            .set_nodelay(self.config.tcp_no_delay)
            .map_err(Error::from)?;

        match Pin::new(conn).poll_read(&mut cx, &mut buf) {
            Poll::Ready(Err(error)) => Err(error),
            Poll::Ready(Ok(bytes)) if bytes == 0 => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection appeared to be closed (EoF)",
            )),
            _ => Ok(()),
        }
        .map_err(Error::from)?;

        Ok(())
    }
}

#[cfg(feature = "h1-rustls")]
#[allow(unused_variables)]
pub(crate) async fn add_tls(
    host: &str,
    stream: TcpStream,
    config: &Config,
) -> Result<TlsStream<TcpStream>, io::Error> {
    let connector: async_rustls::TlsConnector = if let Some(tls_config) =
        config.tls_config.as_ref().cloned()
    {
        tls_config.into()
    } else {
        let mut root_certs = rustls_crate::RootCertStore::empty();
        root_certs.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls_crate::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let config = rustls_crate::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_certs)
            .with_no_client_auth();
        Arc::new(config).into()
    };

    connector
        .connect(
            host.try_into()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            stream,
        )
        .await
}

#[cfg(all(feature = "h1-native-tls", not(feature = "h1-rustls")))]
#[allow(unused_variables)]
pub(crate) async fn add_tls(
    host: &str,
    stream: TcpStream,
    config: &Config,
) -> Result<TlsStream<TcpStream>, async_native_tls::Error> {
    let connector = config.tls_config.as_ref().cloned().unwrap_or_default();

    connector.connect(host, stream).await
}
