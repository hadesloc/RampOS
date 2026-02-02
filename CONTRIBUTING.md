# Contributing to RampOS

Thank you for your interest in contributing to RampOS! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Documentation](#documentation)

---

## Code of Conduct

This project follows a Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to conduct@rampos.io.

### Our Standards

- Be respectful and inclusive
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

---

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.75+ | Backend development |
| Node.js | 18+ | Frontend and SDK development |
| PostgreSQL | 16+ | Database |
| Redis | 7+ | Caching |
| Docker | 24+ | Container runtime |
| Foundry | Latest | Smart contract development |

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:

```bash
git clone https://github.com/YOUR_USERNAME/rampos.git
cd rampos
```

3. Add the upstream remote:

```bash
git remote add upstream https://github.com/rampos/rampos.git
```

---

## Development Setup

### Backend (Rust)

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install sqlx-cli for migrations
cargo install sqlx-cli --no-default-features --features postgres

# Copy environment configuration
cp .env.example .env

# Start infrastructure services
docker-compose up -d postgres redis nats

# Run database migrations
sqlx migrate run

# Build the project
cargo build

# Run tests
cargo test

# Run the API server
cargo run --package ramp-api
```

### Frontend (TypeScript/React)

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev
```

### Smart Contracts (Solidity)

```bash
# Navigate to contracts directory
cd contracts

# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install dependencies
forge install

# Run tests
forge test

# Build contracts
forge build
```

### TypeScript SDK

```bash
# Navigate to SDK directory
cd sdk

# Install dependencies
npm install

# Run tests
npm test

# Build
npm run build
```

---

## Code Style

### Rust

We follow the official Rust style guidelines with some project-specific rules:

```bash
# Format code
cargo fmt

# Run clippy lints
cargo clippy -- -D warnings
```

**Guidelines:**
- Use `thiserror` for error types
- Prefer `anyhow` for application errors, `Result<T, E>` for library code
- Document all public APIs with doc comments
- Use `#[must_use]` for functions that return values that should not be ignored

### TypeScript

```bash
# Format and lint
npm run lint
npm run format
```

**Guidelines:**
- Use TypeScript strict mode
- Prefer `interface` over `type` for object shapes
- Use `async/await` over raw Promises
- Document public functions with JSDoc

### Solidity

```bash
# Format
forge fmt
```

**Guidelines:**
- Follow Solidity style guide
- Use NatSpec comments for public functions
- All external functions must have access control
- Use custom errors instead of require strings

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

**Examples:**

```bash
feat(api): add rate limiting middleware
fix(ledger): correct balance calculation for concurrent transactions
docs(readme): update installation instructions
test(compliance): add AML rule engine tests
```

---

## Making Changes

### Branch Naming

Use descriptive branch names:

```
feature/add-session-keys
fix/ledger-balance-calculation
docs/api-reference-update
refactor/state-machine-cleanup
```

### Development Workflow

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git checkout main
   git merge upstream/main
   ```

2. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes**:
   - Write code
   - Add tests
   - Update documentation

4. **Test your changes**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

5. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat(scope): description"
   ```

6. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

7. **Open a Pull Request**

---

## Pull Request Process

### Before Submitting

- [ ] All tests pass locally
- [ ] Code is formatted (`cargo fmt`, `npm run format`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation is updated if needed
- [ ] Commit messages follow conventions
- [ ] PR description explains the changes

### PR Template

When opening a PR, please include:

```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe how you tested the changes.

## Checklist
- [ ] Tests pass
- [ ] Documentation updated
- [ ] No breaking changes (or documented if any)
```

### Review Process

1. Automated checks run (CI, linting, tests)
2. Code review by maintainers
3. Address feedback and update PR
4. Approval and merge

### After Merge

- Delete your feature branch
- Sync your fork with upstream

---

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test --package ramp-core

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_payin_flow
```

### Frontend Tests

```bash
cd frontend
npm test

# With coverage
npm run test:coverage
```

### Smart Contract Tests

```bash
cd contracts
forge test

# With verbosity
forge test -vvv

# Run specific test
forge test --match-test testPaymaster
```

### Integration Tests

```bash
# Start services
docker-compose up -d

# Run integration tests
cargo test --features integration
```

---

## Documentation

### Code Documentation

- **Rust**: Use `///` doc comments for public items
- **TypeScript**: Use JSDoc comments
- **Solidity**: Use NatSpec comments

### Example (Rust):

```rust
/// Creates a new pay-in intent for the specified user.
///
/// # Arguments
///
/// * `user_id` - The unique identifier of the user
/// * `amount` - The amount in VND
///
/// # Returns
///
/// Returns the created intent on success, or an error if validation fails.
///
/// # Example
///
/// ```rust
/// let intent = create_payin_intent("usr_123", 1000000).await?;
/// ```
pub async fn create_payin_intent(
    user_id: &str,
    amount: i64,
) -> Result<PayinIntent, Error> {
    // implementation
}
```

### Updating Documentation

- API changes: Update `docs/API.md`
- Architecture changes: Update `docs/architecture.md`
- New features: Add to relevant documentation files
- Breaking changes: Document in CHANGELOG.md

---

## Getting Help

- **Discord**: [discord.gg/rampos](https://discord.gg/rampos)
- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and ideas

---

## Recognition

Contributors are recognized in:
- CHANGELOG.md for significant contributions
- GitHub contributors page
- Release notes

Thank you for contributing to RampOS!
