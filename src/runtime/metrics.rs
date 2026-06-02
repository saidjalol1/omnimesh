//! Prometheus metrics for OMNI-MESH runtime monitoring
//!
//! This module provides comprehensive metrics collection for:
//! - Message counters (sent, received, delivered, dropped)
//! - Latency histograms (p50, p95, p99)
//! - Buffer utilization gauges
//! - DTN store size
//! - Transport statistics
//! - Error rates

#[cfg(feature = "prometheus")]
use prometheus::{
    Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, Opts, Registry,
    TextEncoder,
};

#[cfg(feature = "prometheus")]
use lazy_static::lazy_static;

#[cfg(feature = "prometheus")]
lazy_static! {
    // Message counters
    pub static ref MESSAGES_SENT: Counter = Counter::new(
        "omnimesh_messages_sent_total",
        "Total number of messages sent"
    ).unwrap();

    pub static ref MESSAGES_RECEIVED: Counter = Counter::new(
        "omnimesh_messages_received_total",
        "Total number of messages received"
    ).unwrap();

    pub static ref MESSAGES_DELIVERED: Counter = Counter::new(
        "omnimesh_messages_delivered_total",
        "Total number of messages delivered (after deduplication)"
    ).unwrap();

    pub static ref MESSAGES_DROPPED: Counter = Counter::new(
        "omnimesh_messages_dropped_total",
        "Total number of messages dropped"
    ).unwrap();

    pub static ref MESSAGES_DUPLICATE: Counter = Counter::new(
        "omnimesh_messages_duplicate_total",
        "Total number of duplicate messages detected"
    ).unwrap();

    // Latency histograms (microseconds)
    pub static ref LATENCY_SEND: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "omnimesh_latency_send_us",
            "Message send latency in microseconds"
        ).buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0])
    ).unwrap();

    pub static ref LATENCY_RECEIVE: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "omnimesh_latency_receive_us",
            "Message receive latency in microseconds"
        ).buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0])
    ).unwrap();

    pub static ref LATENCY_VERIFY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "omnimesh_latency_verify_us",
            "Signature verification latency in microseconds"
        ).buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0])
    ).unwrap();

    pub static ref LATENCY_SIGN: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "omnimesh_latency_sign_us",
            "Signature generation latency in microseconds"
        ).buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0])
    ).unwrap();

    // Buffer utilization
    pub static ref BUFFER_UTILIZATION: GaugeVec = GaugeVec::new(
        Opts::new(
            "omnimesh_buffer_utilization_percent",
            "Buffer utilization percentage by type"
        ),
        &["buffer_type"]
    ).unwrap();

    // DTN store metrics
    pub static ref DTN_STORE_SIZE: Gauge = Gauge::new(
        "omnimesh_dtn_store_size_bytes",
        "DTN store size in bytes"
    ).unwrap();

    pub static ref DTN_STORE_MESSAGES: Gauge = Gauge::new(
        "omnimesh_dtn_store_messages_total",
        "Number of messages in DTN store"
    ).unwrap();

    // Transport statistics
    pub static ref TRANSPORT_SEND_FAILURES: CounterVec = CounterVec::new(
        Opts::new(
            "omnimesh_transport_send_failures_total",
            "Transport send failures by transport type"
        ),
        &["transport"]
    ).unwrap();

    pub static ref TRANSPORT_BACKPRESSURE: CounterVec = CounterVec::new(
        Opts::new(
            "omnimesh_transport_backpressure_total",
            "Backpressure events by transport type"
        ),
        &["transport"]
    ).unwrap();

    pub static ref TRANSPORT_RECONNECTIONS: CounterVec = CounterVec::new(
        Opts::new(
            "omnimesh_transport_reconnections_total",
            "Reconnection attempts by transport type"
        ),
        &["transport"]
    ).unwrap();

    pub static ref TRANSPORT_CONNECTIONS: GaugeVec = GaugeVec::new(
        Opts::new(
            "omnimesh_transport_connections_active",
            "Active connections by transport type"
        ),
        &["transport"]
    ).unwrap();

    // Error counters
    pub static ref ERRORS_VERIFICATION: Counter = Counter::new(
        "omnimesh_errors_verification_total",
        "Signature verification errors"
    ).unwrap();

    pub static ref ERRORS_SERIALIZATION: Counter = Counter::new(
        "omnimesh_errors_serialization_total",
        "Serialization/deserialization errors"
    ).unwrap();

    pub static ref ERRORS_STORAGE: Counter = Counter::new(
        "omnimesh_errors_storage_total",
        "Storage operation errors"
    ).unwrap();

    // Registry
    pub static ref REGISTRY: Registry = {
        let r = Registry::new();

        // Register all metrics
        r.register(Box::new(MESSAGES_SENT.clone())).unwrap();
        r.register(Box::new(MESSAGES_RECEIVED.clone())).unwrap();
        r.register(Box::new(MESSAGES_DELIVERED.clone())).unwrap();
        r.register(Box::new(MESSAGES_DROPPED.clone())).unwrap();
        r.register(Box::new(MESSAGES_DUPLICATE.clone())).unwrap();

        r.register(Box::new(LATENCY_SEND.clone())).unwrap();
        r.register(Box::new(LATENCY_RECEIVE.clone())).unwrap();
        r.register(Box::new(LATENCY_VERIFY.clone())).unwrap();
        r.register(Box::new(LATENCY_SIGN.clone())).unwrap();

        r.register(Box::new(BUFFER_UTILIZATION.clone())).unwrap();

        r.register(Box::new(DTN_STORE_SIZE.clone())).unwrap();
        r.register(Box::new(DTN_STORE_MESSAGES.clone())).unwrap();

        r.register(Box::new(TRANSPORT_SEND_FAILURES.clone())).unwrap();
        r.register(Box::new(TRANSPORT_BACKPRESSURE.clone())).unwrap();
        r.register(Box::new(TRANSPORT_RECONNECTIONS.clone())).unwrap();
        r.register(Box::new(TRANSPORT_CONNECTIONS.clone())).unwrap();

        r.register(Box::new(ERRORS_VERIFICATION.clone())).unwrap();
        r.register(Box::new(ERRORS_SERIALIZATION.clone())).unwrap();
        r.register(Box::new(ERRORS_STORAGE.clone())).unwrap();

        r
    };
}

/// Exports metrics in Prometheus text format
#[cfg(feature = "prometheus")]
pub fn export_metrics() -> Result<String, String> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();

    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| format!("Failed to encode metrics: {}", e))?;

    String::from_utf8(buffer).map_err(|e| format!("Failed to convert metrics to string: {}", e))
}

/// HTTP server for Prometheus metrics scraping
#[cfg(all(feature = "prometheus", feature = "tokio"))]
pub mod server {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Starts HTTP server for Prometheus metrics on the given port
    pub async fn start_metrics_server(port: u16) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind metrics server: {}", e))?;

        println!(
            "Prometheus metrics server started on http://{}/metrics",
            addr
        );

        loop {
            match listener.accept().await {
                Ok((mut socket, _)) => {
                    tokio::spawn(async move {
                        let mut buffer = [0u8; 1024];

                        if let Ok(n) = socket.read(&mut buffer).await {
                            let request = String::from_utf8_lossy(&buffer[..n]);

                            if request.contains("GET /metrics") {
                                match export_metrics() {
                                    Ok(metrics) => {
                                        let response = format!(
                                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
                                            metrics.len(),
                                            metrics
                                        );
                                        let _ = socket.write_all(response.as_bytes()).await;
                                    }
                                    Err(e) => {
                                        let error_response = format!(
                                            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\r\n{}",
                                            e.len(),
                                            e
                                        );
                                        let _ = socket.write_all(error_response.as_bytes()).await;
                                    }
                                }
                            } else {
                                let response =
                                    "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

// No-op implementations when prometheus feature is disabled
#[cfg(not(feature = "prometheus"))]
pub fn export_metrics() -> Result<String, String> {
    Err("Prometheus metrics not enabled".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "prometheus")]
    fn test_metrics_export() {
        use super::*;

        // Increment some counters
        MESSAGES_SENT.inc();
        MESSAGES_RECEIVED.inc_by(5.0);

        // Record some latencies
        LATENCY_SEND.observe(150.0);
        LATENCY_VERIFY.observe(75.0);

        // Set some gauges
        DTN_STORE_MESSAGES.set(42.0);
        BUFFER_UTILIZATION.with_label_values(&["send"]).set(65.0);

        // Export metrics
        let metrics = export_metrics().expect("Failed to export metrics");

        assert!(metrics.contains("omnimesh_messages_sent_total"));
        assert!(metrics.contains("omnimesh_messages_received_total"));
        assert!(metrics.contains("omnimesh_latency_send_us"));
        assert!(metrics.contains("omnimesh_dtn_store_messages_total"));

        println!("Exported metrics:\n{}", metrics);
    }
}
