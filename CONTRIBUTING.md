# Contributing to Pandabox

Thank you for considering contributing to Pandabox! We appreciate your time and effort in making this project better. This document outlines the process for contributing to the project and what to expect when getting involved.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report any unacceptable behavior to the project maintainers.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the [issue tracker](https://github.com/mastrHyperion98/Pandabox/issues) to see if the problem has already been reported. If it hasn't, please open a new issue with the following information:

- A clear, descriptive title
- A detailed description of the issue
- Steps to reproduce the problem
- Expected behavior vs actual behavior
- Screenshots or logs if applicable
- Your operating system and Rust version

### Suggesting Enhancements

We welcome suggestions for new features and improvements. Please open an issue with the following information:

- A clear, descriptive title
- A detailed description of the enhancement
- Why this enhancement would be useful
- Any alternative solutions you've considered

### Pull Requests

1. **Fork** the repository and create your branch from `main`
2. **Test** your changes thoroughly
3. **Document** any new features or significant changes
4. **Update** the CHANGELOG.md if applicable
5. **Run** the test suite and ensure all tests pass
6. **Lint** your code with `cargo clippy` and format with `cargo fmt`
7. **Submit** your pull request with a clear description of the changes

## Development Setup

### Prerequisites

- Rust (latest stable version)
- Cargo
- SQLite development libraries

### Building the Project

```bash
# Clone the repository
git clone https://github.com/mastrHyperion98/Pandabox.git
cd Pandabox

# Build in development mode
cargo build

# Run tests
cargo test

# Run clippy for linting
cargo clippy -- -D warnings

# Format the code
cargo fmt -- --check
```

### Coding Standards

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Write clear, concise commit messages (see [Conventional Commits](https://www.conventionalcommits.org/))
- Document all public APIs with rustdoc comments
- Keep functions small and focused on a single responsibility
- Write tests for new functionality

### Testing

- Unit tests should be placed in the same file as the code they test
- Integration tests should be placed in the `tests/` directory
- Ensure all tests pass before submitting a pull request
- Add tests for any new functionality or bug fixes

## Security

### Reporting Security Vulnerabilities

If you discover a security vulnerability in Pandabox, please report it responsibly by emailing [your-email@example.com] with a detailed description. Do not disclose the issue publicly until it has been addressed.

### Security Best Practices

- Never hardcode sensitive information
- Use Rust's type system to enforce security guarantees
- Follow the principle of least privilege
- Sanitize all user inputs
- Use constant-time operations for cryptographic operations

## Code Review Process

1. A maintainer will review your pull request
2. You may receive feedback or be asked to make changes
3. Once approved, your changes will be merged into the main branch
4. Your contribution will be included in the next release

## Recognition

All contributors will be recognized in the project's CHANGELOG.md and README.md. Significant contributions may also be eligible for commit access to the repository.

## Questions?

If you have any questions about contributing, feel free to open an issue or reach out to the maintainers.
