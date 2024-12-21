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

This is the architecture we tend to achieve
