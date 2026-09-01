# 🛠️ Development & Contribution Guide for `fly-common`

## 🧪 Testing & Linting

```bash
# Run unit & integration tests
cargo test

# Run linter with zero warnings
cargo clippy --all-targets -- -D warnings
```

## 📦 Local Development with Downstream Apps

When developing new features in `fly-common` and testing them in an application (like `bList` or `fly-app-template`):

Add a Cargo patch to the downstream app's `Cargo.toml`:

```toml
[dependencies]
fly_common = { git = "https://github.com/radmuffin/fly-common" }

[patch."https://github.com/radmuffin/fly-common"]
fly_common = { path = "../fly-common" }
```

Cargo will use your local working copy during development and compile from Git in CI/CD.
