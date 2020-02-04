//! Types and traits for http clients.
//!
//! This crate has been extracted from `surf`'s internals, but can be used by any http client impl.
//! The purpose of this crate is to provide a unified interface for multiple HTTP client backends,
//! so that they can be abstracted over without doing extra work.

#![forbid(future_incompatible, rust_2018_idioms)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, missing_doc_code_examples, unreachable_pub)]
#![cfg_attr(feature = "docs", feature(doc_cfg))]
// Forbid `unsafe` for the native & curl features, but allow it (for now) under the WASM backend
#![cfg_attr(
    not(all(feature = "wasm_client", target_arch = "wasm32")),
    forbid(unsafe_code)
)]

use futures::future::BoxFuture;
use futures::io::{AsyncRead, Cursor};

use std::fmt::{self, Debug};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg_attr(feature = "docs", doc(cfg(curl_client)))]
#[cfg(all(feature = "curl_client", not(target_arch = "wasm32")))]
pub mod isahc;

#[cfg_attr(feature = "docs", doc(cfg(wasm_client)))]
#[cfg(all(feature = "wasm_client", target_arch = "wasm32"))]
pub mod wasm;

#[cfg_attr(feature = "docs", doc(cfg(native_client)))]
#[cfg(feature = "native_client")]
pub mod native;

/// An HTTP Request type with a streaming body.
pub type Request = http::Request<Body>;

/// An HTTP Response type with a streaming body.
pub type Response = http::Response<Body>;

/// An error returned by the HTTP backend
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// An abstract HTTP client.
///
/// __note that this is only exposed for use in middleware. Building new backing clients is not
/// recommended yet. Once it is we'll likely publish a new `http_client` crate, and re-export this
/// trait from there together with all existing HTTP client implementations.__
///
/// ## Spawning new request from middleware
/// When threading the trait through a layer of middleware, the middleware must be able to perform
/// new requests. In order to enable this we pass an `HttpClient` instance through the middleware,
/// with a `Clone` implementation. In order to spawn a new request, `clone` is called, and a new
/// request is enabled.
///
/// How `Clone` is implemented is up to the implementors, but in an ideal scenario combining this
/// with the `Client` builder will allow for high connection reuse, improving latency.
pub trait HttpClient: Debug + Unpin + Send + Sync + Clone + 'static + Sized {
    /// Perform a request.
    fn send(&self, req: Request) -> BoxFuture<'static, Result<Response, Error>>;
}

/// The raw body of an http request or response.
///
/// A body is a stream of `Bytes` values, which are shared handles to byte buffers.
/// Both `Body` and `Bytes` values can be easily created from standard owned byte buffer types
/// like `Vec<u8>` or `String`, using the `From` trait.
pub struct Body {
    reader: Option<Box<dyn AsyncRead + Unpin + Send + 'static>>,
    /// Intentionally use `u64` over `usize` here.
    /// `usize` won't work if you try to send 10GB file from 32bit host.
    #[allow(dead_code)] // not all backends make use of this
    len: Option<u64>,
}

impl Body {
    /// Create a new empty body.
    pub fn empty() -> Self {
        Self {
            reader: None,
            len: Some(0),
        }
    }

    /// Create a new instance from a reader.
    pub fn from_reader(reader: impl AsyncRead + Unpin + Send + 'static) -> Self {
        Self {
            reader: Some(Box::new(reader)),
            len: None,
        }
    }

    /// Validate that the body was created with `Body::empty()`.
    pub fn is_empty(&self) -> bool {
        self.reader.is_none()
    }
}

impl AsyncRead for Body {
    #[allow(missing_doc_code_examples)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.reader.as_mut() {
            Some(reader) => Pin::new(reader).poll_read(cx, buf),
            None => Poll::Ready(Ok(0)),
        }
    }
}

impl fmt::Debug for Body {
    #[allow(missing_doc_code_examples)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body").field("reader", &"<hidden>").finish()
    }
}

impl From<Vec<u8>> for Body {
    #[allow(missing_doc_code_examples)]
    #[inline]
    fn from(vec: Vec<u8>) -> Body {
        let len = vec.len() as u64;
        Self {
            reader: Some(Box::new(Cursor::new(vec))),
            len: Some(len),
        }
    }
}

impl<R: AsyncRead + Unpin + Send + 'static> From<Box<R>> for Body {
    /// Converts an `AsyncRead` into a Body.
    #[allow(missing_doc_code_examples)]
    fn from(reader: Box<R>) -> Self {
        Self {
            reader: Some(reader),
            len: None,
        }
    }
}
