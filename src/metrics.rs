use prometheus::{Registry, GaugeVec, Opts};
use lazy_static::lazy_static;


lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref CONTAINER_CPU: GaugeVec = GaugeVec::new(
        Opts::new("container_cpu_usage", "CPU usage per container"),
        &["container"]
    ).unwrap();
    pub static ref CONTAINER_MEM: GaugeVec = GaugeVec::new(
        Opts::new("container_memory_usage", "Memory usage per container (in MB)"),
        &["container"]
    ).unwrap();
}