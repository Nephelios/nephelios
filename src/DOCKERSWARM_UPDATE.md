# Nephelios and Docker Swarm Integration

## Overview

Nephelios leverages **Docker Swarm** to manage and deploy applications in a distributed manner. By integrating Swarm, Nephelios can automatically distribute application replicas across available nodes, ensuring scalability, fault tolerance, and efficient resource utilization.

## Why Docker Swarm?

Docker Swarm is a **lightweight orchestration tool** that allows Nephelios to:

- 🖥 **Distribute application instances** automatically across multiple nodes.
- 🔄 **Ensure high availability** by handling node failures.
- 📈 **Scale applications dynamically** by adjusting the number of replicas.
- 🔧 **Simplify networking** with built-in overlay networks.

## How Nephelios Uses Docker Swarm

### **1️⃣ Swarm Initialization**

Nephelios checks whether Docker Swarm is initialized. If not, it automatically initializes Swarm with the appropriate manager node.

🚀 **Implementation:**

- Function: `init_swarm(ip_address: &str)`
- Uses `docker swarm init --advertise-addr <ip_address>`
- Ensures that the Swarm cluster is correctly set up before deploying applications.

### **2️⃣ Deploying Applications in Swarm**

Nephelios uses `docker stack deploy` to launch applications in Swarm mode. Each application is defined using a **Docker Compose file**.

🚀 **Implementation:**

- Function: `deploy_nephelios_stack(app_name: &str, compose_path: &str)`
- Runs: `docker stack deploy -c <compose_path> <app_name>`
- Ensures that the application is distributed across the cluster.

### **3️⃣ Service Management**

Nephelios provides features to **start, stop, and remove services** within the Swarm cluster.

🚀 **Implementation:**

- **Stopping a service:** `docker service rm <service_name>`
- **Scaling a service:** `docker service scale <service_name>=<replica_count>`
- **Checking active services:** `docker service ls`

### **4️⃣ Load Balancing with Traefik**

Nephelios uses **Traefik** as a reverse proxy to dynamically route traffic to deployed services.

🚀 **Implementation:**

- **Automatic service discovery** using Docker labels.
- **TLS certificates** managed via Let's Encrypt.
- **Traffic routing** based on hostnames and paths.

### **5️⃣ Monitoring and Pruning**

To maintain efficiency, Nephelios **monitors and prunes** unused Docker images.

🚀 **Implementation:**

- Function: `prune_images()`
- Runs: `docker image prune -af`
- Ensures that unused resources do not consume storage.

## Example Deployment Flow

1️⃣ **Initialize Swarm** (if not already active)

```sh
cargo run
```

2️⃣ **Deploy an application**

```sh
POST /create
{
  "app_name": "my-app",
  "app_type": "nodejs",
  "github_url": "https://github.com/user/repo"
}
```

3️⃣ **Check deployed services**

```sh
docker service ls
```

4️⃣ **Scale the application**

```sh
docker service scale my-app=3
```

5️⃣ **Remove the application**

```sh
POST /remove
{
  "app_name": "my-app"
}
```

## Conclusion

Nephelios' integration with Docker Swarm ensures **automated, scalable, and resilient application deployment**. With features like **auto-initialization, service scaling, and load balancing**, Nephelios transforms application deployment into a seamless process.

