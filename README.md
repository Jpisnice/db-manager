# �️ Database Manager

A powerful, secure, and user-friendly Rust-based database management tool that simplifies database container orchestration using Docker. Create, manage, and connect to PostgreSQL, MySQL, and Redis databases with ease through an intuitive Terminal User Interface (TUI).

## ✨ Features

### 🗃️ **Multi-Database Support**
- **PostgreSQL 15** - Full-featured relational database
- **MySQL 8.0** - Popular relational database with comprehensive features
- **Redis 7** - High-performance in-memory data structure store

### 🐳 **Docker Integration**
- Automatic container creation and lifecycle management
- Health checks and status monitoring
- Volume management for persistent data storage
- Automatic port assignment with defaults (PostgreSQL: 5432, MySQL: 3306, Redis: 6379)

### 🔐 **Security & Encryption**
- **ChaCha20Poly1305** encryption for all sensitive data
- **Scrypt** key derivation for secure passphrase hashing
- Encrypted credential storage with salted hashes
- Master passphrase protection for all configurations
- **Password reset functionality** - never get locked out permanently

### 🖥️ **Interactive Terminal UI**
- Beautiful, responsive Terminal User Interface powered by Ratatui
- Real-time visual feedback with cursor indicators
- Color-coded interface with yellow highlighting for active fields
- Cross-platform terminal support (Linux, macOS, Windows)
- Intuitive navigation with arrow keys and shortcuts

### 📊 **Database Management**
- Create databases with guided step-by-step wizard
- View detailed database information and connection strings
- Delete databases with confirmation prompts
- Real-time database list with refresh capabilities
- Container status monitoring and health checks

### 🛠️ **Advanced Features**
- **Command-line interface** for automation and scripting
- **Configuration reset** functionality (F1 key or `--reset` flag)
- **Help system** with contextual keyboard shortcuts
- **Error handling** with user-friendly messages
- **Status notifications** with popup overlays

## 🚀 Quick Start

### Prerequisites

- **Rust** (1.70 or later) - [Install Rust](https://rustup.rs/)
- **Docker** (running and accessible) - [Install Docker](https://docs.docker.com/get-docker/)
- **Git** - [Install Git](https://git-scm.com/downloads)

### Installation

1. **Clone the repository**:
```bash
git clone https://github.com/your-username/db-manager.git
cd db-manager
```

2. **Build the project**:
```bash
cargo build --release
```

3. **Run the application**:
```bash
cargo run
# or use the binary directly
./target/release/db-tool
```

## 📖 Usage Guide

### 🔑 First Time Setup

1. **Launch the application** and you'll see the authentication screen
2. **Enter a secure passphrase** - this will be your master password
3. **Remember your passphrase** - it encrypts all your database configurations

### 🎯 Main Menu Navigation

- **📋 List Databases** - View all configured databases
- **➕ Create Database** - Add a new database with guided wizard
- **🔄 Refresh** - Reload database list from configuration
- **❌ Exit** - Quit the application

### 🔧 Creating a Database

1. Select **"Create Database"** from the main menu
2. Follow the step-by-step wizard:
   - **Name**: Enter a unique name for your database
   - **Type**: Choose PostgreSQL (1), MySQL (2), or Redis (3)
   - **Username**: Database user credentials
   - **Password**: Secure password for the user
   - **Database**: Database name (skipped for Redis)
   - **Port**: Default ports auto-set, customize if needed
   - **Root Password**: MySQL root password (MySQL only)
   - **Confirm**: Review and create

### 🔍 Database Details

- **View Information**: Container ID, connection strings, creation dates
- **Copy Connection Strings**: Ready-to-use connection URLs
- **Delete Databases**: Remove with confirmation (press 'd')

## 🆘 Password Recovery

If you forget your passphrase, you have several recovery options:

### Method 1: Interactive Reset (F1 Key)
1. On the login screen, press **F1**
2. Confirm the reset when prompted
3. All configurations will be deleted, allowing a fresh start

### Method 2: Command Line Reset
```bash
cargo run -- --reset
# or
./target/release/db-tool --reset
```

### Method 3: Manual Configuration Removal
```bash
# Linux/macOS
rm -f ~/.config/dbmanager/config.json

# Windows
del %APPDATA%\dbmanager\config.json
```

## 🎨 User Interface

### Color Scheme
- **🟡 Yellow**: Active input fields and highlights
- **🔵 Cyan**: Titles and headers
- **🟢 Green**: Success messages and confirmations
- **🔴 Red**: Errors and warnings
- **⚪ White**: Regular text content
- **⚫ Gray**: Help text and secondary information

### Keyboard Shortcuts

#### Global
- **Esc**: Go back / Quit application
- **Enter**: Confirm / Next step
- **↑↓**: Navigate menus and lists
- **F1**: Reset configuration (login screen only)

#### Database Creation
- **1, 2, 3**: Select database type (PostgreSQL, MySQL, Redis)
- **Tab**: Cycle through database types
- **Backspace**: Delete characters
- **Numbers**: Port input (digits only)

#### Database List
- **c**: Create new database
- **r**: Refresh database list
- **d**: Delete selected database (in details view)

## 🏗️ Architecture

### Project Structure
```
db-manager/
├── src/
│   ├── main.rs           # TUI application and main logic
│   ├── credentials/      # Encryption and credential management
│   │   └── mod.rs        # ChaCha20Poly1305 + Scrypt implementation
│   ├── database/         # Database type definitions and templates
│   │   └── mod.rs        # PostgreSQL, MySQL, Redis configurations
│   └── docker/           # Docker container management
│       └── mod.rs        # Container lifecycle and health checks
├── Cargo.toml           # Dependencies and metadata
├── Cargo.lock           # Dependency lock file
└── README.md           # This documentation
```

### Key Technologies

- **🦀 Rust**: Systems programming language for performance and safety
- **🖼️ Ratatui**: Modern terminal UI framework
- **🔧 Crossterm**: Cross-platform terminal manipulation
- **🐳 Shiplift**: Docker API client for container management
- **⚡ Tokio**: Asynchronous runtime for non-blocking operations
- **🔒 ChaCha20Poly1305**: AEAD encryption for data protection
- **🧂 Scrypt**: Key derivation function for passphrase security

## 🔗 Connection Examples

### Generated Connection Strings

#### PostgreSQL
```
postgresql://username:password@localhost:5432/database_name
```

#### MySQL
```
mysql://username:password@localhost:3306/database_name
```

#### Redis
```
redis://localhost:6379
```

### Using with Popular Clients

#### PostgreSQL (psql)
```bash
psql postgresql://username:password@localhost:5432/database_name
```

#### MySQL (mysql client)
```bash
mysql -h localhost -P 3306 -u username -p database_name
```

#### Redis (redis-cli)
```bash
redis-cli -h localhost -p 6379
```

## 🧪 Development

### Building from Source
```bash
# Development build with debug symbols
cargo build

# Optimized release build
cargo build --release

# Run tests
cargo test

# Run with detailed logging
RUST_LOG=debug cargo run

# Check code formatting
cargo fmt --check

# Run linting
cargo clippy
```

### Command Line Options
```bash
# Show help
cargo run -- --help

# Reset configuration
cargo run -- --reset

# Normal interactive mode (default)
cargo run
```

## 🛡️ Security Features

### Encryption Details
- **Algorithm**: ChaCha20Poly1305 (AEAD - Authenticated Encryption with Associated Data)
- **Key Derivation**: Scrypt with random 32-byte salt
- **Storage**: All sensitive data encrypted at rest
- **Passphrase**: Never stored in plain text, only hashed with salt

### Security Best Practices
- Credentials are encrypted before writing to disk
- Container isolation provides additional security layers
- No sensitive information in logs or debug output
- Secure random number generation for salts and nonces

## 🐛 Troubleshooting

### Common Issues

#### Docker Problems
```bash
# Check if Docker is running
docker version

# Test Docker connectivity
docker ps

# Fix permission issues (Linux)
sudo usermod -aG docker $USER
# Then logout and login again
```

#### Port Conflicts
- Default ports may be in use by other services
- The tool will show errors if containers fail to start
- Try different ports in the creation wizard

#### Authentication Issues
- Use the reset functionality if you forget your passphrase
- Configuration is stored in platform-specific directories:
  - Linux: `~/.config/dbmanager/config.json`
  - macOS: `~/Library/Application Support/dbmanager/config.json`
  - Windows: `%APPDATA%\dbmanager\config.json`

### Health Check Details
- **PostgreSQL**: Uses `pg_isready -U username` for health verification
- **MySQL**: Uses `mysqladmin ping` to check server status  
- **Redis**: Uses `redis-cli ping` for connectivity testing

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork** the repository on GitHub
2. **Clone** your fork locally
3. **Create** a feature branch: `git checkout -b feature/amazing-feature`
4. **Make** your changes and add tests if applicable
5. **Test** your changes: `cargo test && cargo clippy`
6. **Commit** your changes: `git commit -m 'Add amazing feature'`
7. **Push** to your branch: `git push origin feature/amazing-feature`
8. **Open** a Pull Request with a clear description

### Development Guidelines
- Follow Rust naming conventions and formatting (`cargo fmt`)
- Add tests for new functionality
- Update documentation for user-facing changes
- Keep commits focused and atomic

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Rust Community** for the amazing ecosystem and tools
- **Docker** for containerization technology
- **Ratatui Team** for the excellent terminal UI framework
- **Contributors** who help improve this project

---

## 📊 Status

**Current Version**: Active Development  
**Rust Version**: 1.70+  
**Platform Support**: Linux, macOS, Windows  
**Container Runtime**: Docker

> **Note**: This project is actively maintained and developed. Features may evolve between versions. Check the [releases](https://github.com/your-username/db-manager/releases) page for the latest updates.

---

**Built with ❤️ and 🦀 Rust**