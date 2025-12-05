//! Stats for http requests.
use crate::rt::ConnectionStats;
use dashmap::DashMap;
use http::Uri;
use lazy_static::lazy_static;
use std::{
    sync::{atomic::AtomicU64, Arc},
    time::{Instant, SystemTime},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
            connection_stats: Some(ConnectionStats::new(Some(now), now_timestamp, None, None)),
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

#[derive(Clone, Debug, PartialEq)]
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
    pub fn get_request_sent(&self) -> Option<Instant> {
        self.http_stats.request_sent_time
    }

    /// Returns the instant when this request approximately started
    pub fn get_response_start(&self) -> Option<Instant> {
        self.http_stats.response_start_time
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

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug)]
/// Connection and request-level stats for a http request.
pub struct RequestStatsInternal {
    redirect: Option<RequestId>,
    http_stats: HttpConnectionStats,
    poll_start: Instant,
    poll_start_timestamp: u128,
    finished: Option<Instant>,
    url: Uri,
    status_code: u16,
    request_body_size: u32,
    certificate: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Absolute duration for a connection stat.
pub struct AbsoluteDuration {
    start: Instant,
    end: Instant,
}

impl AbsoluteDuration {
    /// Constructor
    pub fn new(start: Instant, end: Instant) -> Self {
        Self { start, end }
    }

    /// Starting instant.
    pub fn start(&self) -> &Instant {
        &self.start
    }

    /// Ending instant.
    pub fn end(&self) -> &Instant {
        &self.end
    }
}

impl Default for RequestStatsInternal {
    fn default() -> Self {
        Self {
            redirect: Default::default(),
            http_stats: Default::default(),
            poll_start: Instant::now(),
            poll_start_timestamp: Default::default(),
            finished: Default::default(),
            url: Default::default(),
            status_code: Default::default(),
            request_body_size: Default::default(),
            certificate: Default::default(),
        }
    }
}

impl RequestStatsInternal {
    /// Sets connection stats.
    pub fn set_connection_stats(&mut self, cs: ConnectionStats) {
        self.http_stats.connection_stats = Some(cs);
    }

    /// Sets TLS stats.
    pub fn set_tls_connect(&mut self, tls_connect: AbsoluteDuration) {
        if let Some(conn_stats) = &mut self.http_stats.connection_stats {
            *conn_stats = ConnectionStats::tls_new(*conn_stats, tls_connect);
        }
    }

    /// Sets the request start time.
    pub fn set_request_sent_time(&mut self, request_sent_time: Instant) {
        self.http_stats.request_sent_time = Some(request_sent_time);
    }

    /// Sets the response start time.
    pub fn set_response_start_time(&mut self, response_start_time: Instant) {
        self.http_stats.response_start_time = Some(response_start_time);
    }

    /// Sets the time the entire request finished.
    pub fn set_finished(&mut self, finished: Instant) -> &mut Self {
        self.finished = Some(finished);
        self
    }

    /// Sets the request ID of the redirect triggered by this request.
    pub fn set_redirect(&mut self, next_req: RequestId) -> &mut Self {
        self.redirect = Some(next_req);
        self
    }

    /// Sets the https status code of this request.
    pub fn set_status_code(&mut self, code: u16) -> &mut Self {
        self.status_code = code;
        self
    }

    /// Sets the time the future for this request started polling; practically, this is
    /// the very first thing that can happen in a request.
    pub fn set_poll_start(&mut self, poll_start: Instant, poll_start_timestamp: u128) -> &mut Self {
        self.poll_start = poll_start;
        self.poll_start_timestamp = poll_start_timestamp;
        self
    }

    /// Sets the url of this request.
    pub fn set_url(&mut self, url: Uri) -> &mut Self {
        self.url = url;
        self
    }

    /// Sets the size of the body for this request.
    pub fn set_request_body_size(&mut self, request_body_size: u32) -> &mut Self {
        self.request_body_size = request_body_size;
        self
    }

    /// Sets the cretificate blob for this request.
    pub fn set_certificate(&mut self, certificate: Option<Vec<u8>>) -> &mut Self {
        self.certificate = certificate;
        self
    }

    /// Gets the request id for the redirect triggered by this request.
    pub fn redirect(&self) -> Option<RequestId> {
        self.redirect.clone()
    }
}

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Gets the next available request id.
pub fn next_request_id() -> RequestId {
    RequestId::new(REQUEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// Returns the current size of the request stats map
pub fn stats_size() -> usize {
    REQUEST_STATS.len()
}

// Get the logical 'finished' time for a request, given that it might
// not have completed sucessfully.
fn extract_finished(s: &RequestStatsInternal) -> Instant {
    if let Some(f) = s.finished {
        return f;
    }

    if let Some(f) = s.http_stats.first_body_byte_time {
        return f;
    }

    if let Some(f) = s.http_stats.response_start_time {
        return f;
    }

    if let Some(f) = s.http_stats.request_sent_time {
        return f;
    }

    if let Some(c) = s.http_stats.connection_stats {
        if let Some(d) = c.get_tls_connect() {
            return d.end;
        }
        if let Some(d) = c.get_connect() {
            return d.end;
        }
        if let Some(d) = c.get_dns_resolve() {
            return d.end;
        }
        if let Some(f) = c.get_start_instant() {
            return f;
        }
    }

    return s.poll_start;
}

/// Retrieve the RequestStats for the specified request id.  The RequestStats object
/// will be empty if the request is not found.  All redirects will be included in the
/// reponse, and subsequent 'consume_request_stats' calls for the same id will return
/// an empty RequestStats object.
pub fn consume_request_stats(req_id: RequestId) -> RequestStats {
    let mut redirects = vec![];

    let mut some_req_id = Some(req_id);

    while let Some(req_id) = some_req_id {
        let Some((_, stats)) = REQUEST_STATS.remove(&req_id.handle.0) else {
            break;
        };

        redirects.push(RedirectStats {
            finished: extract_finished(&stats),
            poll_start: stats.poll_start,
            poll_start_timestamp: stats.poll_start_timestamp,
            http_stats: stats.http_stats,
            status_code: stats.status_code,
            url: stats.url,
            request_body_size: stats.request_body_size,
            certificate: stats.certificate,
        });

        some_req_id = stats.redirect;
    }

    RequestStats { redirects }
}

/// Get the current RequestStatsInternal for the specified ID.
pub fn get_request_stats<'a>(
    req_id: &RequestId,
) -> dashmap::mapref::one::RefMut<'a, u64, RequestStatsInternal> {
    REQUEST_STATS.entry(req_id.handle.0).or_default()
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
/// The unique id for a request.
pub struct RequestId {
    handle: Arc<RequestIdHandle>,
}
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct RequestIdHandle(u64);

impl Drop for RequestIdHandle {
    fn drop(&mut self) {
        REQUEST_STATS.remove(&self.0);
    }
}

impl RequestId {
    fn new(value: u64) -> Self {
        Self {
            handle: Arc::new(RequestIdHandle(value)),
        }
    }
}

lazy_static! {
    static ref REQUEST_STATS: dashmap::DashMap<u64, RequestStatsInternal> =
        DashMap::with_capacity(5000);
}
