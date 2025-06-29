# DB Tool 🛠️

A powerful Rust-based database management tool that simplifies database container orchestration using Docker. Create, manage, and connect to PostgreSQL, MySQL, and Redis databases with ease through both command-line and terminal user interface.

## Features

✨ **Multi-Database Support**
- PostgreSQL 15
- MySQL 8.0
- Redis 7 (Alpine)

🐳 **Docker Integration**
- Automatic container creation and management
- Health checks and status monitoring
- Volume management for data persistence

🔐 **Security**
- Encrypted credential storage
- Secure passphrase-based authentication
- Safe configuration management

🖥️ **User Interface**
- Terminal User Interface (TUI) powered by Ratatui
- Cross-platform terminal support
- Interactive database management

📦 **Container Management**
- Start/stop database containers
- Monitor container health
- Automatic port mapping

## Prerequisites

- **Rust** (1.70 or later)
- **Docker** (running and accessible)
- **Linux/macOS/Windows** (cross-platform support)

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/your-username/db-tool.git
cd db-tool
```

2. Build the project:
```bash
cargo build --release
```

3. Run the tool:
```bash
cargo run
# or
./target/release/db-tool
```

## Quick Start

1. **Launch the application**:
```bash
cargo run
```

2. **Create a new database**:
   - Select "Create Database" from the main menu
   - Choose database type (PostgreSQL, MySQL, or Redis)
   - Provide database name and credentials
   - The tool will automatically pull the Docker image and create the container

3. **Connect to your database**:
   - Use the provided connection string
   - Or connect through the integrated management interface

## Supported Databases

### PostgreSQL
- **Image**: `postgres:15`
- **Default Port**: 5432
- **Features**: Full PostgreSQL functionality with persistent data storage

### MySQL
- **Image**: `mysql:8.0`
- **Default Port**: 3306
- **Features**: Complete MySQL server with root and user management

### Redis
- **Image**: `redis:7-alpine`
- **Default Port**: 6379
- **Features**: In-memory data structure store with persistence

## Configuration

The tool stores encrypted configurations including:
- Database credentials (username, password, database name)
- Container information
- Connection details
- Creation timestamps

All sensitive data is encrypted using a master passphrase that you set during first use.

## Project Structure

```
db-tool/
├── src/
│   ├── main.rs           # Application entry point and TUI
│   ├── credentials/      # Credential management and encryption
│   │   └── mod.rs
│   ├── database/         # Database templates and configurations
│   │   └── mod.rs
│   └── docker/           # Docker container management
│       └── mod.rs
├── Cargo.toml           # Dependencies and project metadata
└── README.md           # This file
```

## Dependencies

The project leverages several high-quality Rust crates:

- **ratatui**: Terminal user interface framework
- **crossterm**: Cross-platform terminal manipulation
- **shiplift**: Docker API client for Rust
- **tokio**: Async runtime
- **serde**: Serialization framework
- **chrono**: Date and time handling
- **anyhow**: Error handling

## Usage Examples

### Creating a PostgreSQL Database
```rust
// The tool handles this through the TUI, but the underlying process:
// 1. Pull postgres:15 image
// 2. Create container with proper environment variables
// 3. Start container and wait for health check
// 4. Store encrypted credentials
```

### Connection Strings Generated
- **PostgreSQL**: `postgresql://username:password@localhost:5432/database`
- **MySQL**: `mysql://username:password@localhost:3306/database`
- **Redis**: `redis://localhost:6379`

## Development

### Building from Source
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request


## Security Considerations

- All credentials are encrypted at rest
- Master passphrase is hashed and salted
- Container isolation provides additional security
- No credentials are logged or stored in plain text

## Troubleshooting

### Docker Issues
- Ensure Docker daemon is running
- Check Docker permissions for your user
- Verify Docker API accessibility

### Container Health Checks
- PostgreSQL: Uses `pg_isready` command
- MySQL: Uses `mysqladmin ping`
- Redis: Uses `redis-cli ping`

### Common Issues
1. **Port conflicts**: Ensure target ports are available
2. **Permission denied**: Check Docker socket permissions
3. **Image pull failures**: Verify internet connection and Docker Hub access

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with ❤️ using Rust
- Powered by Docker for containerization
- UI framework provided by Ratatui
- Special thanks to the Rust community

---

**Note**: This tool is currently in active development. Features and APIs may change between versions. Please check the changelog for breaking changes.