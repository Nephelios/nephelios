use lazy_static::lazy_static;
use prometheus::{GaugeVec, Opts, Registry};

/// Prometheus metrics and registry definitions for Docker container monitoring.
/// This block initializes the custom Prometheus metrics used to track per-container
/// CPU usage, memory usage, and network I/O, as well as the main metrics registry.
lazy_static! {
    /// Global Prometheus registry used to register all custom metrics.
    pub static ref REGISTRY: Registry = Registry::new();
    /// Gauge vector tracking CPU usage per container.
    ///
    /// Metric name: `container_cpu_usage`  
    /// Labels: `container`
    ///
    /// Represents the current CPU usage of each container as a floating-point value.
    pub static ref CONTAINER_CPU: GaugeVec = GaugeVec::new(
        Opts::new("container_cpu_usage", "CPU usage per container"),
        &["container"]
    )
    .unwrap();
    /// Gauge vector tracking memory usage per container.
    ///
    /// Metric name: `container_memory_usage`  
    /// Labels: `container`
    ///
    /// Represents the memory usage of each container, typically in megabytes (MB).
    pub static ref CONTAINER_MEM: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_memory_usage",
            "Memory usage per container (in MB)"
        ),
        &["container"]
    )
    .unwrap();
    /// Gauge vector tracking network input per container.
    ///
    /// Metric name: `container_network_in`  
    /// Labels: `container`
    ///
    /// Represents the total inbound network traffic for each container, in kilobytes (KB).
    pub static ref CONTAINER_NET_IN: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_network_in",
            "Network input per container (in KB)"
        ),
        &["container"]
    )
    .unwrap();
    /// Gauge vector tracking network output per container.
    ///
    /// Metric name: `container_network_out`  
    /// Labels: `container`
    ///
    /// Represents the total outbound network traffic for each container, in kilobytes (KB).
    pub static ref CONTAINER_NET_OUT: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_network_out",
            "Network output per container (in KB)"
        ),
        &["container"]
    )
    .unwrap();
}
