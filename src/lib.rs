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

#[cfg_attr(feature = "docs", doc(cfg(curl_client)))]
#[cfg(all(feature = "curl_client", not(target_arch = "wasm32")))]
pub mod isahc;

#[cfg_attr(feature = "docs", doc(cfg(wasm_client)))]
#[cfg(all(feature = "wasm_client", target_arch = "wasm32"))]
pub mod wasm;

#[cfg_attr(feature = "docs", doc(cfg(native_client)))]
#[cfg(feature = "native_client")]
pub mod native;

#[cfg_attr(feature = "docs", doc(cfg(h1_client)))]
#[cfg(feature = "h1_client")]
pub mod h1;

/// An HTTP Request type with a streaming body.
pub type Request = http_types::Request;

/// An HTTP Response type with a streaming body.
pub type Response = http_types::Response;

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
pub trait HttpClient: std::fmt::Debug + Unpin + Send + Sync + Clone + 'static {
    /// The associated error type.
    type Error: Send + Sync + Into<Error>;

    /// Perform a request.
    fn send(&self, req: Request) -> BoxFuture<'static, Result<Response, Self::Error>>;
}

/// The raw body of an http request or response.
pub type Body = http_types::Body;

/// Error type.
pub type Error = http_types::Error;
