# Agent Guide (yaas)

## What this repo actually is

- Single Rust crate at repo root (`Cargo.toml` package/binary name is `yass`), not a Cargo workspace.
- Main app entrypoint is `src/main.rs`; server wiring is in `src/run.rs`; route composition is in `src/web/routes.rs`.
- Frontend assets live under `frontend/` and are bundled by Vite into `frontend/public/assets/bundles/`.

## Setup that must happen before `cargo run`

- Build frontend assets first (required by config loader):
  - `cd frontend && npm ci`
  - `cd frontend && npm run build:assets`
- `FRONTEND_DIR` must point to the `frontend` directory containing `public/assets/bundles/.vite/manifest.json`.
- Required env vars: `SERVER_ADDRESS`, `HTTPS`, `FRONTEND_DIR`, `DATABASE_DIR`, `JWT_SECRET`.
- Optional env vars: `SUPERUSER_SETUP_KEY`, `CAPTCHA_SITE_KEY`, `CAPTCHA_API_KEY`, `GA_TAG_ID`.
- `.env` is optional (autoloaded by `dotenvy`); if missing, app uses process env.

## Database gotchas

- Runtime does not apply SQL migrations.
- App opens `DATABASE_DIR/default/yaas.db` directly.
- Tests are the only place migrations are auto-applied (`src/test.rs` embeds `db/migrations/*.sql`).
- For local startup, provide a DB with schema already created (e.g. checked-in `build/db/default/yaas.db` or manually apply `db/migrations/*.sql`).

## High-value commands

- Run app: `cargo run`
- Run all tests: `cargo test`
- Run one test by filter: `cargo test <substring>`
- Format Rust: `cargo fmt --all`
- Lint Rust: `cargo clippy --all-targets --all-features`
- Build frontend once: `cd frontend && npm run build:assets`
- Watch frontend bundles: `cd frontend && npm run watch:assets`
- Format frontend files (Biome): `cd frontend && npx biome check --write .`

## CI facts worth mirroring locally

- CI uses Node `24` for frontend builds.
- CI runs `cargo test --workspace` and `cargo build --release --locked` (workspace flag is harmless here because there is one package).
