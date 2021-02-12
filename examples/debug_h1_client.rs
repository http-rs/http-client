#![cfg(feature = "h1_client")]

use http_client::{h1::H1Client, HttpClient};
use http_types::{Method, Request};

#[async_std::main]
async fn main() {
    let client = H1Client::new();

    let req = Request::new(Method::Get, "http://example.org");

    client.send(req).await.unwrap();

    dbg!(client);
}
