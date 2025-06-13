#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(test, deny(rust_2018_idioms))]
#![cfg_attr(all(test, feature = "full"), deny(unreachable_pub))]
#![cfg_attr(all(test, feature = "full"), deny(warnings))]
#![cfg_attr(all(test, feature = "nightly"), feature(test))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # hyper
//!
//! hyper is a **fast** and **correct** HTTP implementation written in and for Rust.
//!
//! ## Features
//!
//! - HTTP/1 and HTTP/2
//! - Asynchronous design
//! - Leading in performance
//! - Tested and **correct**
//! - Extensive production use
//! - [Client](client/index.html) and [Server](server/index.html) APIs
//!
//! If just starting out, **check out the [Guides](https://hyper.rs/guides/1/)
//! first.**
//!
//! ## "Low-level"
//!
//! hyper is a lower-level HTTP library, meant to be a building block
//! for libraries and applications.
//!
//! If looking for just a convenient HTTP client, consider the
//! [reqwest](https://crates.io/crates/reqwest) crate.
//!
//! # Optional Features
//!
//! hyper uses a set of [feature flags] to reduce the amount of compiled code.
//! It is possible to just enable certain features over others. By default,
//! hyper does not enable any features but allows one to enable a subset for
//! their use case. Below is a list of the available feature flags. You may
//! also notice above each function, struct and trait there is listed one or
//! more feature flags that are required for that item to be used.
//!
//! If you are new to hyper it is possible to enable the `full` feature flag
//! which will enable all public APIs. Beware though that this will pull in
//! many extra dependencies that you may not need.
//!
//! The following optional features are available:
//!
//! - `http1`: Enables HTTP/1 support.
//! - `http2`: Enables HTTP/2 support.
//! - `client`: Enables the HTTP `client`.
//! - `server`: Enables the HTTP `server`.
//!
//! [feature flags]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section
//!
//! ## Unstable Features
//!
//! hyper includes a set of unstable optional features that can be enabled through the use of a
//! feature flag and a [configuration flag].
//!
//! The following is a list of feature flags and their corresponding `RUSTFLAG`:
//!
//! - `ffi`: Enables C API for hyper `hyper_unstable_ffi`.
//! - `tracing`: Enables debug logging with `hyper_unstable_tracing`.
//!
//! For example:
//!
//! ```notrust
//! RUSTFLAGS="--cfg hyper_unstable_tracing" cargo build
//! ```
//!
//! [configuration flag]: https://doc.rust-lang.org/reference/conditional-compilation.html
//!
//! # Stability
//!
//! It's worth talking a bit about the stability of hyper. hyper's API follows
//! [SemVer](https://semver.org). Breaking changes will only be introduced in
//! major versions, if ever. New additions to the API, such as new types,
//! methods, or traits will only be added in minor versions.
//!
//! Some parts of hyper are documented as NOT being part of the stable API. The
//! following is a brief list, you can read more about each one in the relevant
//! part of the documentation.
//!
//! - Downcasting error types from `Error::source()` is not considered stable.
//! - Private dependencies use of global variables is not considered stable.
//!   So, if a dependency uses `log` or `tracing`, hyper doesn't promise it
//!   will continue to do so.
//! - Behavior from default options is not stable. hyper reserves the right to
//!   add new options that are enabled by default which might alter the
//!   behavior, for the purposes of protection. It is also possible to _change_
//!   what the default options are set to, also in efforts to protect the
//!   most people possible.
use std::fmt::Display;

#[doc(hidden)]
pub use http;

#[cfg(all(test, feature = "nightly"))]
extern crate test;

#[doc(no_inline)]
pub use http::{header, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};

pub use crate::error::{Error, Result};
use crate::rt::ConnectionStats;

#[derive(Clone, Copy, Debug)]
/// Http-related request stats (including connection stats)
pub struct HttpConnectionStats {
    /// The approximate instant the first body byte was received.
    pub first_body_byte_time: Option<std::time::Instant>,

    /// The approximate instant the first header byte was received.
    pub first_header_byte_time: Option<std::time::Instant>,

    /// The connection stats for this http request (if the connection was
    /// not pooled.)
    pub connection_stats: Option<ConnectionStats>,
}

impl HttpConnectionStats {
    /// Constructs a mostly-empty RequestStats struct, with an instantaneous connection time.  
    /// We can use that to figure out how many http2 requests we are making.
    pub fn new_http2() -> Self {
        let now = std::time::Instant::now();
        Self {
            connection_stats: Some(ConnectionStats {
                start_time: Some(now),
                connect_start: Some(now),
                connect_end: Some(now),
                ..Default::default()
            }),
            first_body_byte_time: None,
            first_header_byte_time: None,
        }
    }
}

impl std::fmt::Display for HttpConnectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.connection_stats {
            c.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
/// Container struct for redirect stats, which are just http connection stats,
/// along with the time the redirect finished.
pub struct RedirectStats {
    /// The approximate instant the redirect finished.
    pub finished: std::time::Instant,

    /// HTTP stats.
    pub connection_stats: HttpConnectionStats,
}

impl std::fmt::Display for RedirectStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.connection_stats.fmt(f)?;

        f.write_fmt(format_args!("next redirect: {:?}", self.finished))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Connection and request-level stats for a http request.
pub struct RequestStats {
    /// Connection-level stats.
    pub http_stats: HttpConnectionStats,

    /// Stats for all the redirects (save the final request.)
    pub redirects: Vec<RedirectStats>,

    /// The approximate moment we started this request.
    pub poll_start: std::time::Instant,

    /// The approximate instant we delivered the response to the caller.
    pub finish: std::time::Instant,
}

impl RequestStats {
    /// Creates an empty RequestStats struct; really only useful for supplying a default
    /// for unsupported http 2 stats.
    pub fn empty() -> Self {
        RequestStats {
            http_stats: HttpConnectionStats {
                first_body_byte_time: None,
                first_header_byte_time: None,
                connection_stats: None,
            },
            redirects: vec![],
            poll_start: std::time::Instant::now(),
            finish: std::time::Instant::now(),
        }
    }

    fn get_request_start(&self) -> std::time::Instant {
        self.poll_start
    }

    /// Returns the time (relative to get_request_start) that the first byte was received
    /// from the server
    pub fn get_header_ttfb(&self) -> Option<core::time::Duration> {
        self.http_stats
            .first_header_byte_time
            .map(|t| t.duration_since(self.get_request_start()))
    }

    /// Gets the time (relative to get_request_start) that the first body byte was received.
    pub fn get_body_ttfb(&self) -> Option<core::time::Duration> {
        self.http_stats
            .first_body_byte_time
            .map(|t| t.duration_since(self.get_request_start()))
    }

    /// Returns the time (relative to get_request_start) that the last redirection
    /// began (this would be the final request made in a chain of redirections)
    pub fn get_last_redirect_start(&self) -> Option<core::time::Duration> {
        self.redirects
            .last()
            .map(|r| r.finished.duration_since(self.get_request_start()))
    }

    /// Returns the time the request end (this does not include body time!)
    pub fn get_request_end(&self) -> core::time::Duration {
        self.finish.duration_since(self.get_request_start())
    }

    /// Sets the instant we started waiting for data from the server.
    pub fn set_poll_start(&mut self, start: std::time::Instant) {
        //        eprintln!("here : {:?}", start.duration_since(self.connection_stats.start_time.unwrap()));
        self.poll_start = start;
    }

    /// Sets the time this request finished.
    pub fn set_finish(&mut self, finish: std::time::Instant) {
        self.finish = finish;
    }
}

impl Display for RequestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for r in &self.redirects {
            r.fmt(f)?;
        }

        if let Some(c) = self.http_stats.connection_stats {
            c.fmt(f)?;
        }

        if let Some(e) = self.get_last_redirect_start() {
            f.write_fmt(format_args!("redirection: {:?}\n", e))?;
        }

        if let Some(e) = self.get_header_ttfb() {
            f.write_fmt(format_args!("time to first header byte: {:?}\n", e))?;
        }

        if let Some(e) = self.get_body_ttfb() {
            f.write_fmt(format_args!("time to first body byte: {:?}\n", e))?;
        }

        f.write_fmt(format_args!("total time: {:?}\n", self.get_request_end()))?;

        Ok(())
    }
}

#[macro_use]
mod cfg;

#[macro_use]
mod trace;

pub mod body;
mod common;
mod error;
pub mod ext;
#[cfg(test)]
mod mock;
pub mod rt;
pub mod service;
pub mod upgrade;

#[cfg(feature = "ffi")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "ffi", hyper_unstable_ffi))))]
pub mod ffi;

cfg_proto! {
    mod headers;
    mod proto;
}

cfg_feature! {
    #![feature = "client"]

    pub mod client;
}

cfg_feature! {
    #![feature = "server"]

    pub mod server;
}
