# Changelog

All notable changes to surf will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

## [6.3.2] - 2021-03-01

_(This was the same thing as 6.3.1 released by git accident.)_

## [6.3.1] - 2021-02-15

### Fixed
- Allow http-client to build & run properly when `h1_client` is enabled without either tls option.
- Prefer `rustls` if both tls features are enabled.

### Internal
- More exhaustive CI for feature combinations.

## [6.3.0] - 2021-02-12

### Added
- Connection pooling (HTTP/1.1 `keep-alive`) for `h1_client` (default).
- `native-tls` (default) and `rustls` feature flags.
    - Only works with `h1_client`.
- Isahc metrics as a response extension for `curl_client`.

### Fixed
- `Box<dyn HttpClient>` no longer infinitely recurses.
- `curl_client` now always correctly reads the response body.
- `hyper_client` should now build correctly.
- `WasmClient` fetch from worker scope now works correctly.

### Internal
- Improved CI

## [6.2.0] - 2020-10-26

This release implements `HttpClient` for `Box<dyn HttpClient>`.

### Added
- `impl HttpClient for Box<dyn HttpClient>`

## [6.1.0] - 2020-10-09

This release brings improvements for `HyperClient` (`hyper_client` feature).

### Added
- `HyperClient` now impls `Default`.
- `HyperClient::from_client(hyper::Client<C>)`.

### Changed
- `HyperClient` now re-uses the internal client, allowing connection pooling.

## [6.0.0] - 2020-09-25

This release moves the responsibility of any client sharing to the user.

### Changed
- `HttpClient` implementations no longer `impl Clone`.
  - The responsibility for sharing is the user's.
- `H1Client` can no longer be instatiated via `H1Client {}`.
  - `::new()` should be used.

## [5.0.1] - 2020-09-18

### Fixed
- Fixed a body stream translation bug in the `hyper_client`.

## [5.0.0] - 2020-09-18

This release includes an optional backend using [hyper.rs](https://hyper.rs/), and uses [async-trait](https://crates.io/crates/async-trait) for `HttpClient`.

### Added
- `hyper_client` feature, for using [hyper.rs](https://hyper.rs/) as the client backend.

### Changed
- `HttpClient` now uses [async-trait](https://crates.io/crates/async-trait).
    - This attribute is also re-exported as `http_client::async_trait`.

### Fixed
- Fixed WASM compilation.
- Fixed Isahc (curl) client translation setting duplicate headers incorrectly.

## [4.0.0] - 2020-07-09

This release allows `HttpClient` to be used as a dynamic Trait object.

- `HttpClient`: removed `Clone` bounds.
- `HttpClient`: removed `Error` type.

## [3.0.0] - 2020-05-29

This patch updates `http-client` to `http-types 2.0.0` and a new version of `async-h1`.

### Changes
- http types and async-h1 for 2.0.0 #27

## [2.0.0] - 2020-04-17

### Added
- Added a new backend: `h1-client` https://github.com/http-rs/http-client/pull/22

### Changed
- All types are now based from `hyperium/http` to `http-types` https://github.com/http-rs/http-client/pull/22
