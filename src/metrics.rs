use lazy_static::lazy_static;
use prometheus::{GaugeVec, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref CONTAINER_CPU: GaugeVec = GaugeVec::new(
        Opts::new("container_cpu_usage", "CPU usage per container"),
        &["container"]
    )
    .unwrap();
    pub static ref CONTAINER_MEM: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_memory_usage",
            "Memory usage per container (in MB)"
        ),
        &["container"]
    )
    .unwrap();
    pub static ref CONTAINER_NET_IN: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_network_in",
            "Network input per container (in KB)"
        ),
        &["container"]
    )
    .unwrap();
    pub static ref CONTAINER_NET_OUT: GaugeVec = GaugeVec::new(
        Opts::new(
            "container_network_out",
            "Network output per container (in KB)"
        ),
        &["container"]
    )
    .unwrap();
}
