# TimeKpr UI Backend

A centralized REST API for managing time controls across multiple Linux computers using [Timekpr-nExT](https://github.com/novakmi/timekpra). This backend provides a modern interface for controlling user time limits, viewing usage statistics, and managing schedules across your local network.

**🎯 For the complete user experience, check out the frontend application: [timekpr-ui-frontend](https://github.com/adambie/timekpr-ui-frontend)** - a React-based web interface that provides an intuitive dashboard for all backend functionality.

## Overview

TimeKpr UI Backend enables centralized management of time controls for Linux systems running Timekpr-nExT. Instead of configuring each computer individually, you can:

- Monitor and control user time limits from a single interface
- View usage statistics and reports across all managed computers
- Schedule time allowances and modifications
- Manage multiple users across different systems
- Apply time changes to offline computers when they come back online

## Technology Stack

This backend is built with modern Rust technologies:

- **[Actix Web](https://actix.rs/)** - High-performance async web framework
- **[SQLx](https://github.com/launchbadge/sqlx)** - Async SQL toolkit with compile-time query checking
- **[SQLite](https://sqlite.org/)** - Lightweight, embedded database
- **[Serde](https://serde.rs/)** - Serialization framework for JSON handling
- **[Tokio](https://tokio.rs/)** - Async runtime for concurrent operations
- **[Utoipa](https://github.com/juhaku/utoipa)** - OpenAPI documentation generation
- **[Argon2](https://github.com/RustCrypto/password-hashes)** - Secure password hashing

## Client Machine Setup

Before using the API, each Linux computer you want to manage must be properly configured:

### 1. Install Timekpr-nExT

Follow the installation instructions from the [official Timekpr-nExT repository](https://github.com/novakmi/timekpra).

For Ubuntu/Debian systems:
```bash
sudo apt update
sudo apt install timekpr-next
```

### 2. Create Management User

Create a dedicated user account that can control Timekpr:

```bash
# Create user
sudo adduser timekpr-remote

# Add to timekpr group
sudo usermod -a -G timekpr timekpr-remote

# Set the same password you'll use for the API admin account
sudo passwd timekpr-remote
```

### 3. Configure SSH Access

Generate SSH keys for passwordless authentication:

```bash
# On the API server machine, generate SSH keys
ssh-keygen -t rsa -b 4096 -f ./ssh/timekpr_key -N ""

# Copy public key to each client machine
ssh-copy-id -i ./ssh/timekpr_key.pub timekpr-remote@CLIENT_IP

# Test connection
ssh -i ./ssh/timekpr_key timekpr-remote@CLIENT_IP "timekpra --help"
```

### 4. Verify Setup

Ensure the `timekpr-remote` user can execute Timekpr commands:

```bash
# Test on client machine
timekpra --userlist
timekpra --userinfo USERNAME
```

## Quick Start with Docker

The easiest way to run the backend is using Docker:

### 1. Clone and Start

```bash
git clone <repository-url>
cd timekpr-ui-backend

# Create necessary directories
mkdir -p instance ssh

# Start the service
docker-compose up -d
```

### 2. Access the API

- **API Endpoint**: http://localhost:5000
- **API Documentation**: http://localhost:5000/swagger-ui/
- **Default Credentials**: admin / admin (change immediately!)

### 3. Configure Environment

Create a `.env` file for production settings:

```bash
cp .env.example .env
# Edit .env with your settings, especially JWT_SECRET
```

## Development Setup

For local development without Docker:

```bash
# Install Rust and dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and setup
git clone <repository-url>
cd timekpr-ui-backend
cp .env.example .env

# Run development server
./scripts/dev.sh run

# Or manually:
cargo run
```

## API Authentication

The API uses JWT tokens for authentication:

1. **Login**: POST `/api/login` with username/password
2. **Use Token**: Include `Authorization: Bearer <token>` in subsequent requests
3. **Change Password**: POST `/api/change-password` to update admin credentials

## Next Steps

Once the backend is running:

1. **Install the frontend**: Get [timekpr-ui-frontend](https://github.com/adambie/timekpr-ui-frontend) for the complete UI experience
2. **Add client machines**: Use the API to register your Timekpr-enabled computers
3. **Configure users**: Set up time limits and schedules for managed users
4. **Monitor usage**: View real-time statistics and reports

For detailed API documentation, visit the Swagger UI at `/swagger-ui/` when the server is running.