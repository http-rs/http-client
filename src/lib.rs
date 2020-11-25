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

#[cfg_attr(feature = "docs", doc(cfg(curl_client)))]
#[cfg(all(feature = "curl_client", not(target_arch = "wasm32")))]
pub mod isahc;

#[cfg_attr(feature = "docs", doc(cfg(wasm_client)))]
#[cfg(all(feature = "wasm_client", target_arch = "wasm32"))]
pub mod wasm;

#[cfg_attr(feature = "docs", doc(cfg(native_client)))]
#[cfg(any(feature = "curl_client", feature = "wasm_client"))]
pub mod native;

#[cfg_attr(feature = "docs", doc(cfg(h1_client)))]
#[cfg(feature = "h1_client")]
pub mod h1;

#[cfg_attr(feature = "docs", doc(cfg(hyper_client)))]
#[cfg(feature = "hyper_client")]
pub mod hyper;

/// An HTTP Request type with a streaming body.
pub type Request = http_types::Request;

/// An HTTP Response type with a streaming body.
pub type Response = http_types::Response;

pub use async_trait::async_trait;
pub use http_types;

/// An abstract HTTP client.
///
/// __note that this is only exposed for use in middleware. Building new backing clients is not
/// recommended yet. Once it is we'll likely publish a new `http_client` crate, and re-export this
/// trait from there together with all existing HTTP client implementations.__
///
/// ## Spawning new request from middleware
///
/// When threading the trait through a layer of middleware, the middleware must be able to perform
/// new requests. In order to enable this efficiently an `HttpClient` instance may want to be passed
/// though middleware for one of its own requests, and in order to do so should be wrapped in an
/// `Rc`/`Arc` to enable reference cloning.
#[async_trait]
pub trait HttpClient: std::fmt::Debug + Unpin + Send + Sync + 'static {
    /// Perform a request.
    async fn send(&self, req: Request) -> Result<Response, Error>;
}

/// The raw body of an http request or response.
pub type Body = http_types::Body;

/// Error type.
pub type Error = http_types::Error;

#[async_trait]
impl HttpClient for Box<dyn HttpClient> {
    async fn send(&self, req: Request) -> Result<Response, Error> {
        self.as_ref().send(req).await
    }
}
