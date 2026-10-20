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
- 📊 Sortable table - click any column header to sort ascending/descending
- ☁️ Optional folder-based cloud sync (zero-knowledge, encrypted blob)

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

## Backup & Sync

Pandabox has an **optional** folder-based cloud sync (off by default). It is zero-knowledge: it writes a single fully-encrypted file (`pandabox.pbx`) into a folder you choose — a Nextcloud/OneDrive/Syncthing/iCloud Drive/Google Drive sync folder, a USB stick, or any local directory. The file is ChaCha20-Poly1305 AEAD ciphertext; nothing readable (not even a record name) ever appears on disk, and the sync provider only ever sees ciphertext.

### How to enable

1. Open **File → Cloud Sync…** in the app.
2. Tick **Enable cloud sync** and choose a sync folder (any folder you have read/write access to).
3. Your vault is written to `<folder>/pandabox.pbx` automatically about 2 seconds after any change (and on exit).

Disabling sync never deletes the file in your folder — it simply stops updating it.

### Restoring on a new machine

On the new machine, open **File → Cloud Sync…** and use the **Restore from a folder** panel: enter the path of the folder containing `pandabox.pbx` and your master password. Your vault is replaced by the file's contents and the app unlocks.

### Conflicts (two machines changed)

If the file in your folder differs from your local vault (e.g. both machines edited it), Pandabox **never overwrites silently**: it shows a prompt where you choose **Import** (take the file) or **Keep local** (the file is left untouched). The side you did not choose is preserved as a backup file `pandabox.pbx.conflict-<timestamp>` so nothing is ever lost. After resolving, choose Import or Keep local to finish.

### Flatpak note

The Flatpak build grants access to your **home directory** only, so the sync folder must live under `~/`. Paths on other mounts (e.g. `/mnt/…`) are not accessible from the sandbox; the native (non-Flatpak) build has no such restriction.

Full design, schema and ADRs: [`docs/cloud-sync/`](./docs/cloud-sync/).

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- Cargo (Rust's package manager)
- SQLite3 development libraries

### Installation

#### Option 1: Install from Flatpak Bundle (Linux - Recommended)

1. Download the latest `pandabox.flatpak` bundle from the [releases page](https://github.com/mastrHyperion98/Pandabox/releases)

2. Install Flatpak if you haven't already:
   ```bash
   # Debian/Ubuntu
   sudo apt install flatpak
   
   # Fedora
   sudo dnf install flatpak
   
   # Arch Linux
   sudo pacman -S flatpak
   ```

3. Install the Pandabox Flatpak bundle:
   ```bash
   flatpak install --user pandabox.flatpak
   ```

4. Run Pandabox:
   ```bash
   flatpak run com.github.mastrHyperion98.Pandabox
   ```
   
   Or launch it from your application menu.

#### Option 2: Build from Source

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

## Design Docs

Design packages for planned and in-progress features live under [`docs/`](./docs/). Currently:

- [`docs/cloud-sync/`](./docs/cloud-sync/) — optional folder-based cloud sync (encrypted vault blob)

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) before submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Icons provided by Anastassiya Motokhova at [flaticon.com](https://www.flaticon.com/authors/anastassiya-motokhova)
- Built with ❤️ using Rust

🔒 **Remember**: Always keep your master password secure and never share it with anyone. The security of your passwords depends on it!
