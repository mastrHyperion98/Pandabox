# Pandabox - Secure Password Manager

Pandabox is a secure, open-source password manager built with Rust, offering robust encryption and a user-friendly interface. It's a complete rewrite of the original Pandabox password manager, focusing on security, performance, and modern development practices.

## Features

- 🔒 Secure password storage using industry-standard encryption
- 🛡️ Master password protection with Argon2 key derivation
- 🔄 Cross-platform compatibility
- 📱 Modern, responsive UI built with Slint
- 🔄 SQLite database for local storage
- 🔑 Secure password generation
- 📝 Secure notes storage
- 🔍 Easy credential management

## Security Architecture

### Encryption Overview

Pandabox employs a multi-layered security approach to protect your sensitive data:

1. **Master Key Derivation**:
   - Uses Argon2id (the winner of the Password Hashing Competition)
   - Memory-hard function to resist GPU/ASIC attacks
   - Configurable work factors to stay ahead of hardware improvements

2. **Data Encryption**:
   - ChaCha20-Poly1305 authenticated encryption
   - 256-bit encryption keys
   - Unique nonce for each encryption operation
   - Authentication tags to detect tampering

3. **Key Management**:
   - Master key never stored on disk
   - Keys derived on-demand from master password
   - Memory is securely zeroed after use
   - Protection against cold-boot attacks

### Security Best Practices

- **Zero Knowledge Architecture**: Your master password never leaves your device
- **Secure Memory Handling**: Sensitive data is wiped from memory when no longer needed
- **Defense in Depth**: Multiple layers of protection against various attack vectors
- **Open Source**: Full transparency with the security community

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- Cargo (Rust's package manager)
- SQLite3 development libraries

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/mastrHyperion98/Pandabox
   cd Pandabox
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run the application:
   ```bash
   cargo run --release
   ```

### First Run

1. On first launch, you'll be prompted to create a new database
2. Choose a strong master password (this will be used to encrypt all your data)
3. The application will create an encrypted database in your home directory
4. Start adding your credentials and secure notes

## Usage

### Adding Credentials
1. Click "Add New" in the main interface
2. Fill in the service details (name, username, password, etc.)
3. Use the built-in password generator if desired
4. Save the entry

### Retrieving Credentials
1. Search for the service or scroll through your list
2. Click on an entry to view details
3. Use the copy buttons to copy usernames or passwords to clipboard

### Security Tips

- Use a strong, unique master password
- Never share your master password
- Keep your system and RustPassLock updated
- Regularly back up your database file
- Use the built-in password generator for strong, unique passwords

## Development

### Building from Source

```bash
# Clone the repository
   git clone https://github.com/mastrHyperion98/Pandabox
   cd Pandabox

# Build in release mode (recommended for production)
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Dependencies

- `slint` - Modern UI toolkit
- `rusqlite` - SQLite database bindings
- `argon2` - Password hashing
- `chacha20poly1305` - Encryption
- `rand` - Cryptographically secure random number generation
- `chrono` - Date and time handling

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) before submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Icons provided by Anastassiya Motokhova at [flaticon.com](https://www.flaticon.com/authors/anastassiya-motokhova)
- Built with ❤️ using Rust

## Security Disclosures

If you discover any security vulnerabilities, please report them by opening an issue or contacting us directly at [your-email@example.com].

---

🔒 **Remember**: Always keep your master password secure and never share it with anyone. The security of your passwords depends on it!
