<h1 align="center">http-client</h1>
<div align="center">
  <strong>
    Types and traits for http clients.
  </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/http-client">
    <img src="https://img.shields.io/crates/v/http-client.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/http-client">
    <img src="https://img.shields.io/crates/d/http-client.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/http-client">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/http-client">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/http-client/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/http-client/blob/main/.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

## Note on intent

This crate is designed to support developer-facing clients instead of being used directly.

If you are looking for a Rust HTTP Client library which can support multiple backend http implementations,
consider using [Surf](https://crates.io/crates/surf), which depends on this library and provides a good developer experience.

## Safety

For non-wasm clients, this crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.


## Feature Flags

This crate does not work without specifying feature flags. No features are set by default.

The following client backends are supported:
- [`async-h1`]() version 1.x, via the `h1-client` feature.
- [`hyper`]() version 0.14.x via the `hyper0_14-client` feature.
- libcurl through [`isahc`]() version 0.9.x via the `isahc0_9-client` feature.
- WASM to JavaScript `fetch` via the `wasm-client` feature.

Additionally TLS support can be enabled by the following options:
- `h1-rustls` uses [`rustls`](https://crates.io/crates/rustls) for the `h1-client`.
- `h1-native-tls` uses OpenSSL for the `h1-client` _(not recommended, no automated testing)_.
- `hyper0_14-rustls` uses [`rustls`](https://crates.io/crates/rustls) for the `hyper0-14-client`.
- `hyper0_14-native-tls` uses OpenSSL for the `hyper0_14-client` _(not recommended, no automated testing)_.
- `isahc0_9-client` (implicit support).
- `wasm-client` (implicit support).

## Contributing

Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

[contributing]: https://github.com/http-rs/http-client/blob/main/.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/http-rs/http-client/labels/good%20first%20issue
[help-wanted]: https://github.com/http-rs/http-client/labels/help%20wanted

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
