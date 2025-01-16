# Nephelios: Our Rust-based platform-as-a-service
Nephelios is platform-as-a-service (PaaS) solution built in Rust, designed to empower developers to effortlessly deploy their applications to the cloud.
Powerful, simple to use, and easily configurable, our platform streamlines the deployment process while offering robust monitoring functionality to track resource consumption in real-time. 
With clear configuration options and detailed insights into application performance, our PaaS ensures developers have both control and visibility at every step.

```
project/
├── src/
│ ├── main.rs # Entry point
│ ├── routes.rs # Define HTTP routes
│ ├── services/
│ │ ├── app\*service.rs # Application lifecycle management
│ │ ├── nginx_service.rs # NGINX configuration management
│ │ ├── helpers/
│ │ │ ├── github_helper.rs # GitHub-related utilities
│ │ │ ├── docker_helper.rs # Docker-related utilities
│ │ │ ├── nginx_helper.rs # NGINX helper functions
│ │ │ └── storage_helper.rs # Database access layer
│ └── models/
│ ├── app_config.rs # Struct definitions (e.g., AppConfig)
│ └── response.rs # Response structures
├── nginx/
│ ├── nginx.conf.template # Template for dynamic NGINX configuration
│ ├── conf.d/
│ │ └── \*.conf # Per-app configurations
├── tests/ # Integration and unit tests
│ ├── test_app_creation.rs # Test creating apps
│ ├── test_routing.rs # Test NGINX routing
│ └── test_docker.rs # Test Docker integration
├── Cargo.toml # Dependencies and project config
├── README.md # Documentation
└── .env # Environment variables
```

This is the architecture we tend to achieve
