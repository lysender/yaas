# Agent Guide (yaas)

This repo is a Rust workspace (edition 2024) with a small Node/Biome frontend bundler.

## Repo Map

- `api/`: Axum REST API server (binary `api`).
- `website/`: Axum + Askama website (binary `website`).
- `db/`: Diesel/Deadpool DB crate.
- `password/`: Password hashing crate.
- `yaas/`: Shared types/utilities + generated protobuf bindings.
- `protogen/`: A CLI binary that generates protobuf payloads and runs ad-hoc HTTP smoke tests.
- `website/frontend/`: Node scripts to build static CSS/JS bundles; formatting via Biome.

## Build / Lint / Test

All Rust commands should be run at repo root unless noted.

### Rust (workspace)

- Build all crates: `cargo build --workspace`
- Run all unit tests: `cargo test --workspace`
- Compile tests only (fast sanity): `cargo test --workspace --no-run`
- Format (rustfmt): `cargo fmt --all`
- Lint (clippy): `cargo clippy --workspace --all-targets --all-features`

### Run binaries

- Run API: `cargo run -p api`
- Run Website: `cargo run -p website`
- Run Protogen CLI: `cargo run -p protogen`

### Run a single Rust test

Cargo supports filtering by substring (module path works well):

- One test by name (workspace): `cargo test --workspace <substring>`
- One test in a crate: `cargo test -p yaas <substring>`
- One test by full path (best): `cargo test -p website pagination::tests::test_next_page`
- One test with output: `cargo test -p api token::tests::test_encode -- --nocapture`

If you need a specific file’s tests, use the module path as the filter.

### Frontend (website assets)

Working dir: `website/frontend/`

- Install deps (CI-friendly): `npm ci`
- Build bundles: `npm run build`
- Format/lint (Biome, writes fixes): `npm run format`

Run Biome on a single file:

- `npx biome check --write path/to/file.js`
  (equivalently: `npm run format -- path/to/file.js`)

Note: `npm run build` writes bundles into `website/frontend/public/assets/bundles/`.

## Configuration / Environment

- `.env` files are ignored (see `.gitignore`). Do not commit secrets.
- Examples exist in `api/.env-example`, `website/.env-example`, `protogen/.env-example`.
- `db/.env` is used for Diesel migrations tooling.

## Cursor / Copilot Rules

- No `.cursorrules`, `.cursor/rules/`, or `.github/copilot-instructions.md` found in this repo.

## Code Style (Rust)

### Formatting

- Follow rustfmt defaults; use `cargo fmt --all`.
- Indentation: 4 spaces for `.rs` and `.proto` (see `.editorconfig`).
- Line endings: LF.

### Imports

- Prefer explicit imports; avoid glob imports except for well-known preludes.
- Order import groups as:
  - `std::...`
  - external crates
  - `crate::...` / `super::...`
- Keep imports minimal; remove unused imports (clippy will complain).

### Types and APIs

- Most crates define `pub type Result<T> = std::result::Result<T, Error>;` in `*/src/error.rs`.
- Use that crate’s `Result<T>` in public APIs/handlers instead of `anyhow::Result`.
- Prefer strong types/DTOs from `yaas::dto` over ad-hoc `HashMap<String, String>`.

### Naming

- Modules/files: `snake_case`.
- Types/traits: `PascalCase`.
- Functions/vars: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.
- Prefer names that encode domain concepts (`OrgApp`, `OauthCode`, `Superuser`).

### Error handling

- Errors use `snafu` (`#[derive(Snafu)]`) across crates.
- When propagating errors:
  - add context with `ResultExt::context(...)` when it improves debugging.
  - validate inputs with `ensure!(..., ValidationSnafu { ... })` (website handlers do this).
- Don’t `unwrap()`/`expect()` in library code; reserve it for binaries/tests where failure is fatal.
- HTTP-facing crates map domain errors to status codes in `*/src/error.rs`.
  - If you add a new error variant that should be user-visible, update the `StatusCode` mapping.

### Logging

- Use `tracing` macros (`info!`, `warn!`, `error!`) rather than `println!`.
- Avoid logging secrets (JWTs, passwords, CAPTCHA keys, connection strings).

### Tests

- Unit tests are typically inline: `mod tests { ... }` with `#[test]`.
- Use `cargo test -p <crate> <filter>` to isolate failures.
- The `protogen` binary runs ad-hoc end-to-end HTTP calls; it requires a running API and valid env.

## Code Style (Website/Frontend)

### JS/CSS/HTML formatting

- Biome config lives at `website/biome.json` (4-space indent, single quotes, semicolons, 120 cols).
- Use `npm run format` to apply formatting + safe fixes + import organization.

### Asset build scripts

- Bundling scripts live in `website/frontend/scripts/`.
- Prefer small, deterministic scripts (no network access during build).
- Fail fast with clear error messages when expected files/config are missing.

## Practical Agent Workflow

- Start with `cargo test --workspace --no-run` after dependency changes.
- Before sending a PR/patch, run:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets --all-features`
  - `cargo test --workspace`
- If you touched frontend assets/scripts, also run in `website/frontend/`:
  - `npm run format`
  - `npm run build`
