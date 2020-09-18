# Changelog

All notable changes to surf will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://book.async.rs/overview/stability-guarantees.html).

## [Unreleased]

## [4.0.0] - 2020-07-09

This release allows `HttpClient` to be used as a dynamic Trait object.

- `HttpClient`: removed `Clone` bounds.
- `HttpClient`: removed `Error` type.

## [3.0.0] - 2020-05-29

This patch updates `http-client` to `http-types 2.0.0` and a new version of `async-h1`.

## Changes
- http types and async-h1 for 2.0.0 #27

## [2.0.0] - 2020-04-17

## Added
- Added a new backend: `h1-client` https://github.com/http-rs/http-client/pull/22

## Changed
- All types are now based from `hyperium/http` to `http-types` https://github.com/http-rs/http-client/pull/22
