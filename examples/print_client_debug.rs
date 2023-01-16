#[cfg(any(
    feature = "h1-client",
    feature = "hyper0_14-client",
    feature = "isahc0_9-client",
    feature = "wasm-client"
))]
use http_client::HttpClient;
#[cfg(any(
    feature = "h1-client",
    feature = "hyper0_14-client",
    feature = "isahc0_9-client",
    feature = "wasm-client"
))]
use http_types::{Method, Request};

#[cfg(feature = "hyper0_14-client")]
use tokio1 as tokio;

#[cfg(any(feature = "h1-client", feature = "docs"))]
use http_client::h1::H1Client as Client;
#[cfg(all(feature = "hyper0_14-client", not(feature = "docs")))]
use http_client::hyper::HyperClient as Client;
#[cfg(all(feature = "isahc0_9-client", not(feature = "docs")))]
use http_client::isahc::IsahcClient as Client;
#[cfg(all(feature = "wasm-client", not(feature = "docs")))]
use http_client::wasm::WasmClient as Client;

#[cfg(any(
    feature = "h1-client",
    feature = "hyper0_14-client",
    feature = "isahc0_9-client",
    feature = "wasm-client"
))]
#[cfg_attr(
    any(
        feature = "h1-client",
        feature = "isahc0_9-client",
        feature = "wasm-client"
    ),
    async_std::main
)]
#[cfg_attr(feature = "hyper0_14-client", tokio::main)]
async fn main() {
    let client = Client::new();

    let mut args = std::env::args();
    args.next(); // ignore binary name
    let arg = args.next();
    println!("{arg:?}");
    let req = Request::new(Method::Get, arg.as_deref().unwrap_or("http://example.org"));

    let response = client.send(req).await.unwrap();
    dbg!(response);

    dbg!(&client);
}

#[cfg(not(any(
    feature = "h1-client",
    feature = "hyper0_14-client",
    feature = "isahc0_9-client",
    feature = "wasm-client"
)))]
fn main() {
    eprintln!("ERROR: A client backend must be select via `--features`: h1-client, hyper0_14-client, isahc0_9-client, wasm-client")
}
