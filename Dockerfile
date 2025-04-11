FROM rust:1.86.0-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/nephelios

# Copy the entire project for building
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN cargo build --release


FROM debian:bookworm-slim AS runtime

# Install runtime dependencies and Docker
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gnupg \
    git \
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && chmod a+r /etc/apt/keyrings/docker.gpg \
    && echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian \
    $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
    tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Create working directory
RUN mkdir -p /app

# Copy binary and configurations
COPY --from=builder /usr/src/nephelios/target/release/nephelios /usr/local/bin/
COPY prometheus.yml /app/config/prometheus/prometheus.yml
COPY ./grafana/ /app/config/grafana/
COPY ./dashboards/ /app/config/dashboards/
COPY nephelios.yml /app/nephelios.yml

WORKDIR /app

CMD ["/usr/local/bin/nephelios"]
