# Contributing to FrutigerOS

Thank you for your interest in FrutigerOS.

## Before You Start

- Read the [README](README.md) and [ROADMAP](ROADMAP.md).
- Check open issues before starting new work.
- For large changes, open an issue to discuss first.

## Development Setup

### AeroWM (Rust)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install system dependencies (Arch Linux)
sudo pacman -S wayland libxkbcommon systemd

# Build
cd aerowm
cargo build
cargo test
```

## Commit Style

Use conventional commits:

```
feat(aerowm): add workspace switching
fix(aeroshell): correct panel height on HiDPI
docs: update AeroWM keybindings
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## Pull Request Process

1. Fork the repository.
2. Create a branch: `git checkout -b feat/your-feature`
3. Make your changes with tests where applicable.
4. Run `cargo test` (for Rust components).
5. Open a PR against `main`.
6. Fill in the PR template completely.

## Code Standards

- Rust: run `cargo clippy` and `cargo fmt` before committing.
- No `unwrap()` in production paths — use proper error handling.
- Document public APIs with doc comments.
- Keep commits focused. One logical change per commit.

## What Not to Do

- Do not claim features work if they are not tested.
- Do not add dependencies without discussion.
- Do not break existing functionality without a migration path.
- Do not submit AI-generated code you have not reviewed and understood.

## Questions

Open a GitHub Discussion or join the community Discord (link in README).
