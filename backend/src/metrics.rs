use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("metric can be created");
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        &["method", "path"],
    )
    .expect("metric can be created");
    pub static ref WEBSOCKET_CONNECTIONS_ACTIVE: IntGauge = IntGauge::new(
        "websocket_connections_active",
        "Number of active WebSocket connections",
    )
    .expect("metric can be created");
    pub static ref REGISTERED_USERS_TOTAL: IntGauge = IntGauge::new(
        "registered_users_total",
        "Total number of registered users",
    )
    .expect("metric can be created");
}

pub fn init_metrics() {
    REGISTRY
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(WEBSOCKET_CONNECTIONS_ACTIVE.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(REGISTERED_USERS_TOTAL.clone()))
        .expect("collector can be registered");
}

pub fn render_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
