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
use std::{fmt::Display, ops::Sub};

#[doc(hidden)]
pub use http;

#[cfg(all(test, feature = "nightly"))]
extern crate test;

#[doc(no_inline)]
pub use http::{header, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};

pub use crate::error::{Error, Result};
use crate::rt::ConnectionStats;

#[derive(Clone, Copy, Debug)]
/// Connection and request-level stats for a http request.
pub struct RequestStats {
    /// Connection-level stats.
    connection_stats: ConnectionStats,

    /// The apprximate instant that we started waiting for actual bytes on the connection.
    poll_start: std::time::Instant,

    /// The approximate instant the first byte of the response payload was received.
    fbt: Option<std::time::Instant>,

    /// The approximate instant we started the very last redirect this request experienced.
    last_redirect: Option<std::time::Instant>,

    /// The approximate instant we delivered the response to the caller.
    finish: Option<std::time::Instant>,
}

impl RequestStats {
    /// Constructs a mostly-empty RequestStats struct, with an instantaneous connection time.  
    /// We can use that to figure out how many http2 requests we are making.
    pub fn new_http2() -> Self {
        let now = std::time::Instant::now();
        Self {
            connection_stats: ConnectionStats {
                start_time: Some(now),
                connect_start: Some(now),
                connect_end: Some(now),
                ..Default::default()
            },
            poll_start: now,
            fbt: None,
            last_redirect: None,
            finish: None,
        }
    }
    /// Returns the time the dns resolve started
    pub fn get_dns_resolve_start(&self) -> Option<core::time::Duration> {
        self.connection_stats.dns_resolve_start.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the dns resolve finished
    pub fn get_dns_resolve_end(&self) -> Option<core::time::Duration> {
        self.connection_stats.dns_resolve_end.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the socket connection was started
    pub fn get_connect_start(&self) -> Option<core::time::Duration> {
        self.connection_stats.connect_start.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the socket finished connecting
    pub fn get_connect_end(&self) -> Option<core::time::Duration> {
        self.connection_stats.connect_end.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the tls negotiation started
    pub fn get_tls_start(&self) -> Option<core::time::Duration> {
        self.connection_stats.tls_connect_start.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the tls negotiation completed
    pub fn get_tls_end(&self) -> Option<core::time::Duration> {
        self.connection_stats.tls_connect_end.map(|t| {
            self.connection_stats
                .start_time
                .map(|start| t.duration_since(start))
        })?
    }

    /// Returns the time the socket was polled for data.  Can be zero, if the
    /// connection was re-used from a pool
    pub fn get_transfer_start(&self) -> core::time::Duration {
        self.connection_stats
            .start_time
            .map(|start| self.poll_start.duration_since(start))
            .unwrap_or(core::time::Duration::from_millis(0))
    }

    /// Returns the time (relative to get_transfer_start) that the first byte was received
    /// from the server
    pub fn get_ttfb(&self) -> Option<core::time::Duration> {
        self.fbt.map(|t| t.duration_since(self.poll_start))
    }

    /// Returns the time (relative to get_transfer_start) that the last redirection
    /// began (this would be the final request made in a chain of redirections)
    pub fn get_last_redirect_start(&self) -> Option<core::time::Duration> {
        self.last_redirect
            .map(|t| t.duration_since(self.poll_start))
    }

    /// Returns the time the request end (this does not include body time!)
    pub fn get_request_end(&self) -> Option<core::time::Duration> {
        self.finish.map(|t| t.duration_since(self.poll_start))
    }

    /// Get connection stats.
    pub fn get_connection_stats(&self) -> ConnectionStats {
        self.connection_stats
    }

    /// Set connection stats.
    pub fn set_connection_stats(&mut self, cs: ConnectionStats) {
        self.connection_stats = cs;
    }

    /// Sets the instant the last redirect started.
    pub fn set_last_redirect(&mut self, redirect: Option<std::time::Instant>) {
        if redirect.is_some() {
            self.last_redirect = redirect;
        }
    }

    /// Sets the instant we started waiting for data from the server.
    pub fn set_poll_start(&mut self, start: std::time::Instant) {
        self.poll_start = start;
    }

    /// Sets the time this request finished.
    pub fn set_finish(&mut self, finish: std::time::Instant) {
        self.finish = Some(finish);
    }
}

impl Display for RequestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = self.get_dns_resolve_start() {
            if let Some(e) = self.get_dns_resolve_end() {
                f.write_fmt(format_args!("name resolution: {:?}\n", e.sub(s)))?;
            }
        }

        if let Some(s) = self.get_connect_start() {
            if let Some(e) = self.get_connect_end() {
                f.write_fmt(format_args!("connection: {:?}\n", e.sub(s)))?;
            }
        }

        if let Some(s) = self.get_tls_start() {
            if let Some(e) = self.get_tls_end() {
                f.write_fmt(format_args!("tls negotiation: {:?}\n", e.sub(s)))?;
            }
        }

        if let Some(e) = self.get_last_redirect_start() {
            f.write_fmt(format_args!("redirection: {:?}\n", e))?;
        }

        if let Some(e) = self.get_ttfb() {
            f.write_fmt(format_args!("time to first byte: {:?}\n", e))?;
        }

        if let Some(e) = self.get_request_end() {
            f.write_fmt(format_args!("total time: {:?}\n", e))?;
        }

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
