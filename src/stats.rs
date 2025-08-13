//! Stats for http requests.
use crate::rt::ConnectionStats;
use http::Uri;
use std::time::{Instant, SystemTime};

#[derive(Clone, Copy, Debug)]
/// Http-related request stats (including connection stats)
pub struct HttpConnectionStats {
    /// The approximate instant the first body byte was received.
    first_body_byte_time: Option<Instant>,

    /// The approximate instant the request went out over the wire.
    request_sent_time: Option<Instant>,

    /// The approximate instant the response became available from the wire.
    response_start_time: Option<Instant>,

    /// The connection stats for this http request (if the connection was
    /// not pooled.)
    connection_stats: Option<ConnectionStats>,
}

impl HttpConnectionStats {
    /// Constructs a new HttpConnectionStats
    pub fn new(
        first_body_byte_time: Option<Instant>,
        connection_stats: Option<ConnectionStats>,
    ) -> Self {
        Self {
            first_body_byte_time,
            connection_stats,
            request_sent_time: None,
            response_start_time: None,
        }
    }
    /// Constructs a mostly-empty RequestStats struct, with an instantaneous connection time.  
    /// We can use that to figure out how many http2 requests we are making.
    pub fn new_http2() -> Self {
        let now = Instant::now();
        let now_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        Self {
            connection_stats: Some(ConnectionStats::new(now, now_timestamp, now, now, now, now)),
            first_body_byte_time: Some(now),
            request_sent_time: None,
            response_start_time: None,
        }
    }

    /// Sets the request and response start times.
    pub fn set_request_times(&mut self, request_sent_time: Instant, response_start_time: Instant) {
        self.request_sent_time = Some(request_sent_time);
        self.response_start_time = Some(response_start_time);
    }

    /// Gets connection stats.
    pub fn get_connection_stats(&self) -> Option<&ConnectionStats> {
        self.connection_stats.as_ref()
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

#[derive(Clone, Debug)]
/// Container struct for redirect stats, which are just http connection stats,
/// along with the time the redirect finished.
pub struct RedirectStats {
    /// The approximate instant the redirect finished.
    finished: Instant,

    /// The approximate instant the redirect started polling.
    poll_start: Instant,

    /// The approximate timestamp (in us) the redirect started polling.
    poll_start_timestamp: u128,

    /// HTTP stats.
    http_stats: HttpConnectionStats,

    /// HTTP status code.
    status_code: u16,

    /// The url of this redirect.
    url: Uri,

    /// Request body size.
    request_body_size: u32,

    /// Leaf DER-encoded certificate, if available.
    certificate: Option<Vec<u8>>,
}

impl RedirectStats {
    /// Construct a new RedirectStats
    pub fn new(
        finished: Instant,
        poll_start: Instant,
        poll_start_timestamp: u128,
        http_stats: HttpConnectionStats,
        status_code: u16,
        url: Uri,
        request_body_size: u32,
        certificate: Option<Vec<u8>>,
    ) -> Self {
        Self {
            finished,
            poll_start,
            poll_start_timestamp,
            http_stats,
            status_code,
            url,
            request_body_size,
            certificate,
        }
    }

    /// Returns the DER-encoded certificate for this redirect, if available.
    pub fn get_certificate_bytes(&self) -> Option<&Vec<u8>> {
        self.certificate.as_ref()
    }

    /// Gets the HTTP status code
    pub fn get_status_code(&self) -> u16 {
        self.status_code
    }

    /// Gets the URL
    pub fn get_url(&self) -> &Uri {
        &self.url
    }

    /// Gets the request body size
    pub fn get_request_body_size(&self) -> u32 {
        self.request_body_size
    }

    /// Get the http connection stats for this request
    pub fn get_http_stats(&self) -> &HttpConnectionStats {
        &self.http_stats
    }

    /// Returns the timestamp (in ms) when this request approximately started
    pub fn get_request_start_timestamp(&self) -> u128 {
        self.poll_start_timestamp
    }

    /// Returns the instant when this request approximately started
    pub fn get_request_start(&self) -> Instant {
        self.poll_start
    }

    /// Returns the instant when this request approximately started
    pub fn get_request_sent(&self) -> Instant {
        self.http_stats.request_sent_time.unwrap()
    }

    /// Returns the instant when this request approximately started
    pub fn get_response_start(&self) -> Instant {
        self.http_stats.response_start_time.unwrap()
    }

    /// Gets the time  that the first body byte was received.
    pub fn get_body_ttfb(&self) -> Option<Instant> {
        self.http_stats.first_body_byte_time
    }

    /// Returns the time the request ended.
    pub fn get_request_end(&self) -> Instant {
        self.finished
    }
}

impl std::fmt::Display for RedirectStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.http_stats.fmt(f)?;

        f.write_fmt(format_args!("next redirect: {:?}", self.finished))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Connection and request-level stats for a http request.
pub struct RequestStats {
    redirects: Vec<RedirectStats>,
}

impl RequestStats {
    /// Create a new request stats object.
    pub fn new(
        http_stats: HttpConnectionStats,
        mut redirects: Vec<RedirectStats>,
        poll_start: Instant,
        poll_start_timestamp: u128,
        finished: Instant,
        url: Uri,
        status_code: u16,
        request_body_size: u32,
        certificate: Option<Vec<u8>>,
    ) -> Self {
        redirects.push(RedirectStats {
            status_code,
            finished,
            poll_start,
            poll_start_timestamp,
            http_stats,
            url,
            request_body_size,
            certificate,
        });
        RequestStats { redirects }
    }

    /// Creates an empty RequestStats struci; really only useful for supplying a default
    /// for unsupported http 2 stats.
    pub fn empty() -> Self {
        RequestStats { redirects: vec![] }
    }

    /// Stats for all the redirects, including the final request.
    /// If there are no redirects, this will contain the one request!
    pub fn redirects(&self) -> &Vec<RedirectStats> {
        &self.redirects
    }
}
