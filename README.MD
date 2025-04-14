# Nephelios Backend

## Overview

Nephelios is a modern cloud platform that simplifies application deployment and management. The backend provides robust APIs and services for deploying, monitoring, and managing your applications in the cloud.

## Features

- **Application Deployment**: Deploy applications directly from GitHub repositories
- **Real-Time Monitoring**: Track application performance and resource usage
- **Scalability**: Scale applications dynamically with Docker Swarm
- **Secure Environment**: Built-in security features for safe deployments
- **Metrics and Dashboards**: Integrated Prometheus and Grafana for monitoring

## Tech Stack

- **Language**: Rust
- **Framework**: Warp (HTTP Server)
- **Containerization**: Docker & Docker Swarm
- **Monitoring**: Prometheus & Grafana
- **Reverse Proxy**: Traefik

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Docker & Docker Compose
- Prometheus & Grafana (optional for monitoring)

### Installation

1. Clone the repository
   ```bash
   git clone https://github.com/Nephelios/nephelios.git
   cd nephelios
   ```

2. Install dependencies
   ```bash
   cargo build --release
   ```

3. Set up environment variables
   ```bash
   # Create a .env file in the root directory and add:
   NEPHELIOS_PORT=3030
   NEPHELIOS_APPS_PORT=5173
   ADVERTISE_ADDR=<your-ip-address>
   LEAVE_SWARM=false
   ```

### Development

Start the development server:

```bash
cargo run
```

The application will be available at `http://localhost:3030`.

### Building for Production

```bash
cargo build --release
```

The built binary will be in the `target/release` directory.

## Docker Deployment

Nephelios backend can be easily deployed using Docker:

1. Build the Docker image:
   ```bash
   docker build -t nephelios-backend .
   ```

2. Run the container:
   ```bash
   docker-compose up
   ```

## Project Structure

```
nephelios/
├── src/
│   ├── routes.rs       # HTTP route definitions
│   ├── services/       # Core services for deployment and monitoring
│   │   ├── helpers/    # Utility functions for Docker, GitHub, etc.
│   │   └── websocket.rs # WebSocket for real-time updates
│   ├── metrics.rs      # Prometheus metrics integration
│   └── main.rs         # Application entry point
├── tests/              # Integration and unit tests
├── docker-compose.yml  # Docker Compose configuration
├── Dockerfile          # Docker configuration
├── prometheus.yml      # Prometheus configuration
├── grafana/            # Grafana provisioning and dashboards
├── nephelios.yml       # Nephelios-specific configuration file
├── dashboards/         # Pre-configured Grafana dashboards
├── .env                # Environment variables file
└── Cargo.toml          # Rust project configuration

```

## Environment Variables

- `NEPHELIOS_PORT`: Port for the backend server (default: `3030`)
- `NEPHELIOS_APPS_PORT`: Port for deployed applications (default: `5173`)
- `ADVERTISE_ADDR`: IP address for Docker Swarm advertisement
- `LEAVE_SWARM`: Whether to leave the Docker Swarm on shutdown (default: `false`)

## 🤝 Contributing

We welcome contributions from the community! To contribute:

1. **Fork the Repository**: Click on the "Fork" button at the top right of this page.

2. **Create a Branch**: Create a new branch for your feature or bugfix.
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make Changes**: Implement your feature or fix.

4. **Commit Changes**: Commit your changes with a descriptive message.
   ```bash
   git commit -m "Add feature: your feature description"
   ```

5. **Push to GitHub**: Push your changes to your forked repository.
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request**: Open a pull request to merge your changes into the main repository.

For detailed guidelines, please refer to our `CONTRIBUTING.md` file.

## 📄 License

_No licence has been chosen yet._

---

*For more information and updates, visit our [GitHub Organization](https://github.com/Nephelios).* 