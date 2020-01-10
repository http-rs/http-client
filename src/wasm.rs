//! http-client implementation for fetch

use super::{Body, HttpClient, Request, Response};

use futures::future::BoxFuture;
use futures::prelude::*;

use std::convert::TryFrom;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

/// WebAssembly HTTP Client.
#[derive(Debug)]
pub struct WasmClient {
    _priv: (),
}

impl WasmClient {
    /// Create a new instance.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Clone for WasmClient {
    fn clone(&self) -> Self {
        Self { _priv: () }
    }
}

impl HttpClient for WasmClient {
    type Error = std::io::Error;

    fn send(&self, req: Request) -> BoxFuture<'static, Result<Response, Self::Error>> {
        let fut = Box::pin(async move {
            let req: fetch::Request = fetch::Request::new(req)?;
            let mut res = req.send().await?;

            let body = res.body_bytes();
            let mut response =
                Response::new(http_types::StatusCode::try_from(res.status()).unwrap());
            response.set_body(Body::from(body));
            for (name, value) in res.headers() {
                let name: http_types::headers::HeaderName = name.parse().unwrap();
                response.insert_header(
                    &name,
                    value.parse::<http_types::headers::HeaderValue>().unwrap(),
                );
            }

            Ok(response)
        });

        Box::pin(InnerFuture { fut })
    }
}

struct InnerFuture {
    fut: Pin<Box<dyn Future<Output = Result<Response, io::Error>> + 'static>>,
}

// This is safe because WASM doesn't have threads yet. Once WASM supports threads we should use a
// thread to park the blocking implementation until it's been completed.
unsafe impl Send for InnerFuture {}

impl Future for InnerFuture {
    type Output = Result<Response, io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // This is safe because we're only using this future as a pass-through for the inner
        // future, in order to implement `Send`. If it's safe to poll the inner future, it's safe
        // to proxy it too.
        unsafe { Pin::new_unchecked(&mut self.fut).poll(cx) }
    }
}

mod fetch {
    use futures_util::io::AsyncReadExt;
    use http::request::Parts;
    use js_sys::{Array, ArrayBuffer, Reflect, Uint8Array};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::window;
    use web_sys::RequestInit;

    use std::io;
    use std::iter::{IntoIterator, Iterator};
    use std::pin::Pin;

    /// Create a new fetch request.

    /// An HTTP Fetch Request.
    pub(crate) struct Request {
        init: RequestInit,
        url: String,
        _body_buf: Pin<Vec<u8>>,
    }

    impl Request {
        /// Create a new instance.
        pub(crate) fn new(req: super::Request) -> Result<Self, io::Error> {
            let (
                Parts {
                    method,
                    uri,
                    headers,
                    ..
                },
                mut body,
            ) = req.into_parts();

            //create a fetch request initaliser
            let mut init = web_sys::RequestInit::new();

            //set the fetch method
            init.method(method.as_ref());

            //add any fetch headers
            let init_headers = web_sys::Headers::new().unwrap();
            for (name, value) in headers.iter() {
                init_headers
                    .append(name.as_str(), value.to_str().unwrap())
                    .map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "could not add header: {} = {}",
                                name.as_str(),
                                value.to_str().expect("could not stringify header value")
                            ),
                        )
                    })?;
            }
            init.headers(&init_headers);

            //convert the body into a uint8 array
            // needs to be pinned and retained inside the Request because the Uint8Array passed to
            // js is just a portal into WASM linear memory, and if the underlying data is moved the
            // js ref will become silently invalid
            let mut body_buf = Vec::with_capacity(1024);
            futures::executor::block_on(body.read_to_end(&mut body_buf)).map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "could not read body into a buffer")
            })?;
            let body_pinned = Pin::new(body_buf);
            if body_pinned.len() > 0 {
                unsafe {
                    let uint_8_array = js_sys::Uint8Array::view(&body_pinned);
                    init.body(Some(&uint_8_array));
                }
            }

            Ok(Self {
                init,
                url: uri.to_string(),
                _body_buf: body_pinned,
            })
        }

        /// Submit a request
        // TODO(yoshuawuyts): turn this into a `Future` impl on `Request` instead.
        pub(crate) async fn send(self) -> Result<Response, io::Error> {
            // Send the request.
            let window = window().expect("A global window object could not be found");
            let request = web_sys::Request::new_with_str_and_init(&self.url, &self.init).unwrap();
            let promise = window.fetch_with_request(&request);
            let resp = JsFuture::from(promise).await.unwrap();
            debug_assert!(resp.is_instance_of::<web_sys::Response>());
            let res: web_sys::Response = resp.dyn_into().unwrap();

            // Get the response body.
            let promise = res.array_buffer().unwrap();
            let resp = JsFuture::from(promise).await.unwrap();
            debug_assert!(resp.is_instance_of::<js_sys::ArrayBuffer>());
            let buf: ArrayBuffer = resp.dyn_into().unwrap();
            let slice = Uint8Array::new(&buf);
            let mut body: Vec<u8> = vec![0; slice.length() as usize];
            slice.copy_to(&mut body);

            Ok(Response::new(res, body))
        }
    }

    /// An HTTP Fetch Response.
    pub(crate) struct Response {
        res: web_sys::Response,
        body: Option<Vec<u8>>,
    }

    impl Response {
        fn new(res: web_sys::Response, body: Vec<u8>) -> Self {
            Self {
                res,
                body: Some(body),
            }
        }

        /// Access the HTTP headers.
        pub(crate) fn headers(&self) -> Headers {
            Headers {
                headers: self.res.headers(),
            }
        }

        /// Get the request body as a byte vector.
        ///
        /// Returns an empty vector if the body has already been consumed.
        pub(crate) fn body_bytes(&mut self) -> Vec<u8> {
            self.body.take().unwrap_or_else(|| vec![])
        }

        /// Get the HTTP return status code.
        pub(crate) fn status(&self) -> u16 {
            self.res.status()
        }
    }

    /// HTTP Headers.
    pub(crate) struct Headers {
        headers: web_sys::Headers,
    }

    impl IntoIterator for Headers {
        type Item = (String, String);
        type IntoIter = HeadersIter;

        fn into_iter(self) -> Self::IntoIter {
            HeadersIter {
                iter: js_sys::try_iter(&self.headers).unwrap().unwrap(),
            }
        }
    }

    /// HTTP Headers Iterator.
    pub(crate) struct HeadersIter {
        iter: js_sys::IntoIter,
    }

    impl Iterator for HeadersIter {
        type Item = (String, String);

        fn next(&mut self) -> Option<Self::Item> {
            let pair = self.iter.next()?;

            let array: Array = pair.unwrap().into();
            let vals = array.values();

            let prop = String::from("value").into();
            let key = Reflect::get(&vals.next().unwrap(), &prop).unwrap();
            let value = Reflect::get(&vals.next().unwrap(), &prop).unwrap();

            Some((
                key.as_string().to_owned().unwrap(),
                value.as_string().to_owned().unwrap(),
            ))
        }
    }
}
