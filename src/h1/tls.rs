use std::fmt::Debug;
use std::net::SocketAddr;
use std::pin::Pin;

use async_std::io::ReadExt;
use async_std::net::TcpStream;
use async_trait::async_trait;
use deadpool::managed::{Manager, Object, RecycleResult};
use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{Context, Poll};

cfg_if::cfg_if! {
    if #[cfg(feature = "rustls")] {
        use async_tls::client::TlsStream;
    } else if #[cfg(feature = "native-tls")] {
        use async_native_tls::TlsStream;
    }
}

use crate::Error;

#[derive(Clone, Debug)]
pub(crate) struct TlsConnection {
    host: String,
    addr: SocketAddr,
}
impl TlsConnection {
    pub(crate) fn new(host: String, addr: SocketAddr) -> Self {
        Self { host, addr }
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
        let tls_stream = add_tls(&self.host, raw_stream).await?;
        Ok(tls_stream)
    }

    async fn recycle(&self, conn: &mut TlsStream<TcpStream>) -> RecycleResult<Error> {
        let mut buf = [0; 4];
        match futures::poll!(conn.get_ref().read(&mut buf)) {
            Poll::Ready(Err(error)) => Err(error),
            _ => Ok(()),
        }.map_err(Error::from)?;
        Ok(())
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "rustls")] {
        async fn add_tls(host: &str, stream: TcpStream) -> Result<TlsStream<TcpStream>, std::io::Error> {
            let connector = async_tls::TlsConnector::default();
            connector.connect(host, stream).await
        }
    } else if #[cfg(feature = "native-tls")] {
        async fn add_tls(
            host: &str,
            stream: TcpStream,
        ) -> Result<TlsStream<TcpStream>, async_native_tls::Error> {
            async_native_tls::connect(host, stream).await
        }
    }
}
