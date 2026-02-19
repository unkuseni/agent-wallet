# Contributing to AI Agent Wallet

Thank you for your interest in contributing to the AI Agent Wallet project! This document provides guidelines and instructions for contributing to the development of autonomous agent wallets on Solana.

## Code of Conduct

We are committed to fostering a welcoming and respectful community. Please:
- Be respectful and inclusive in all communications
- Focus on constructive feedback and technical discussions
- Help create a safe environment for everyone, regardless of background or experience level

## Getting Started

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Solana CLI 1.17+ (install via [solana-install](https://docs.solana.com/cli/install-solana-cli-tools))
- Git
- Basic understanding of Solana blockchain concepts

### Development Setup
1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/agent-wallet.git
   cd agent-wallet
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/original-owner/agent-wallet.git
   ```
4. **Install development tools**:
   ```bash
   cargo install cargo-watch
   cargo install cargo-tarpaulin  # For code coverage
   cargo install cargo-audit      # For security auditing
   cargo install cargo-deny       # For dependency checking
   ```

5. **Build the project**:
   ```bash
   cargo build
   ```

6. **Run tests**:
   ```bash
   cargo test
   ```

## Development Workflow

### Branch Strategy
- `main`: Stable, production-ready code
- `develop`: Integration branch for features
- `feature/*`: New features or enhancements
- `bugfix/*`: Bug fixes
- `release/*`: Release preparation

### Creating a Feature
1. **Create a feature branch** from `develop`:
   ```bash
   git checkout develop
   git pull upstream develop
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following coding standards below.

3. **Write tests** for all new functionality.

4. **Update documentation** including README, API docs, and examples.

5. **Commit your changes** with descriptive commit messages:
   ```bash
   git add .
   git commit -m "feat: add support for multi-signature wallets"
   ```

   Use conventional commit format:
   - `feat:` New feature
   - `fix:` Bug fix
   - `docs:` Documentation only
   - `style:` Formatting, missing semicolons, etc.
   - `refactor:` Code refactoring
   - `test:` Adding or updating tests
   - `chore:` Maintenance tasks

6. **Keep your branch updated**:
   ```bash
   git fetch upstream
   git rebase upstream/develop
   ```

### Pull Request Process
1. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub from your branch to `develop`.

3. **Fill out the PR template** including:
   - Description of changes
   - Related issues
   - Testing performed
   - Screenshots (if UI changes)
   - Checklist of requirements

4. **Address review feedback** promptly.

5. **Ensure all checks pass**:
   - CI builds
   - Tests
   - Linting
   - Security scans

6. **Squash commits** if requested by maintainers.

7. **Wait for approval** from at least one maintainer.

## Coding Standards

### Rust Guidelines
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for consistent formatting:
  ```bash
  cargo fmt --all
  ```
- Run `clippy` for linting:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- Prefer `unwrap()` only in tests; use proper error handling in production code
- Document public APIs with rustdoc comments:
  ```rust
  /// Creates a new wallet with encrypted key storage.
  ///
  /// # Arguments
  /// * `name` - Human-readable name for the wallet
  /// * `passphrase` - Encryption passphrase (minimum 12 characters)
  ///
  /// # Returns
  /// `Result<Wallet>` - The created wallet or an error
  pub fn create_wallet(name: &str, passphrase: &str) -> Result<Wallet> {
      // implementation
  }
  ```

### Solana-Specific Standards
- Use `solana_sdk` and `solana_client` crates from crates.io
- Handle transaction errors gracefully with retry logic
- Validate all inputs before constructing transactions
- Use appropriate commitment levels for different operations
- Follow security best practices for key management

### File Organization
- Place new modules in appropriate directories (`core/`, `agent/`, `dapp/`)
- Keep files focused and under 500 lines when possible
- Use `mod.rs` or `lib.rs` for module declarations
- Group related functionality in the same module

### Naming Conventions
- `PascalCase` for types, traits, and enums
- `snake_case` for variables, functions, and modules
- `SCREAMING_SNAKE_CASE` for constants
- Use descriptive names that indicate purpose

## Testing Requirements

### Test Types
1. **Unit Tests**: Test individual functions and methods
   - Place in same file as code being tested (`#[cfg(test)] mod tests`)
   - Mock external dependencies

2. **Integration Tests**: Test component interactions
   - Place in `tests/` directory
   - Use `solana-program-test` for blockchain interactions

3. **End-to-End Tests**: Test full workflows on devnet
   - Use separate test wallets with minimal funds
   - Clean up test artifacts after completion

### Test Guidelines
- Write tests before or alongside code (TDD encouraged)
- Aim for >80% code coverage
- Test error conditions and edge cases
- Use descriptive test names that explain the scenario
- Avoid testing implementation details; test behavior

### Running Tests
```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --ignore-tests

# Run integration tests
cargo test --test integration

# Run specific test
cargo test test_wallet_creation
```

## Documentation

### Code Documentation
- Document all public APIs with rustdoc
- Explain complex algorithms with comments
- Include examples in documentation when helpful
- Keep comments up-to-date with code changes

### Project Documentation
- Update `README.md` for significant changes
- Update `SKILLS.md` for agent-facing capability changes
- Update `ROADMAP.md` for timeline adjustments
- Add or update examples in `examples/` directory

### Architecture Documentation
- Update architecture diagrams for structural changes
- Document design decisions in `docs/design-decisions.md`
- Update security considerations for new features

## Security Considerations

### Critical Areas
- **Key Management**: Never log or expose private keys
- **Transaction Validation**: Validate all transactions before signing
- **Input Sanitization**: Sanitize all user/agent inputs
- **Error Messages**: Don't leak sensitive information in errors

### Security Checklist
- [ ] No hardcoded secrets in code
- [ ] Input validation for all parameters
- [ ] Proper error handling without information leakage
- [ ] Use of secure cryptographic libraries
- [ ] Security audit for new cryptographic code
- [ ] Rate limiting implemented where appropriate

### Reporting Security Issues
For security vulnerabilities, please **DO NOT** create a public issue. Instead:
1. Email security@agent-wallet.org with details
2. We will acknowledge receipt within 48 hours
3. We will work with you to understand and fix the issue
4. We will coordinate disclosure after the fix is released

## Dependencies

### Adding Dependencies
1. Check if dependency is necessary and well-maintained
2. Prefer minimal dependencies to reduce attack surface
3. Ensure license compatibility (Apache 2.0 or MIT)
4. Add to appropriate `Cargo.toml` with version pinning:
   ```toml
   [dependencies]
   solana-sdk = "=1.17.0"  # Pin exact version
   ```

### Dependency Auditing
Regularly audit dependencies:
```bash
cargo audit
cargo deny check
```

## Performance Considerations

### Optimization Guidelines
- Profile before optimizing
- Focus on critical paths (transaction signing, RPC calls)
- Use appropriate data structures
- Implement caching where beneficial
- Avoid unnecessary clones and allocations

### Benchmarking
Add benchmarks for performance-critical code:
```rust
#[bench]
fn bench_wallet_creation(b: &mut Bencher) {
    b.iter(|| Wallet::new("test", "passphrase"));
}
```

Run benchmarks with:
```bash
cargo bench
```

## Questions and Help

### Resources
- [Rust Documentation](https://doc.rust-lang.org/)
- [Solana Documentation](https://docs.solana.com/)
- [Project Documentation](docs/)
- [GitHub Issues](https://github.com/original-owner/agent-wallet/issues)

### Getting Help
1. Check existing documentation and issues
2. Search closed issues for similar questions
3. Ask in GitHub Discussions (if enabled)
4. Create an issue for bugs or feature requests

### Community Channels
- Discord: [link-to-discord]
- Twitter: [@agentwallet]
- Blog: [blog.agent-wallet.org]

## Recognition

Contributors will be recognized in:
- GitHub contributors list
- Release notes
- Project documentation (if significant contribution)
- Optional: Contributor profile on project website

---

Thank you for contributing to the future of autonomous AI agents on Solana! Your work helps build the infrastructure for the next generation of decentralized applications.

*Last Updated: [Current Date]*