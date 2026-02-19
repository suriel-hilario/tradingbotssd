## ADDED Requirements

### Requirement: CI workflow on every push and pull request
A GitHub Actions workflow SHALL run on every push to any branch and on every pull request targeting `main`. It SHALL execute in order: format check, linting, and tests. Any step failure SHALL fail the workflow and block PR merge.

#### Scenario: Format check fails
- **WHEN** `cargo fmt --check` detects unformatted code
- **THEN** the workflow fails with a clear error and subsequent steps are skipped

#### Scenario: Clippy lint violation found
- **WHEN** `cargo clippy -- -D warnings` emits any warning
- **THEN** the workflow fails and the PR cannot be merged until warnings are resolved

#### Scenario: Unit or integration test fails
- **WHEN** `cargo test` exits with a non-zero code
- **THEN** the workflow fails and the failing test name is visible in the GitHub Actions log

#### Scenario: All CI checks pass
- **WHEN** fmt, clippy, and tests all succeed
- **THEN** the workflow reports success and the PR is eligible for merge

---

### Requirement: CD workflow on merge to main
A separate GitHub Actions workflow SHALL trigger exclusively on pushes to the `main` branch (i.e., merged PRs). It SHALL build a release binary, deploy it to the Droplet, and restart the systemd service.

#### Scenario: Cross-compiled musl binary built
- **WHEN** the CD workflow starts
- **THEN** it produces a `clawbot` binary targeting `x86_64-unknown-linux-musl` using `cross` or `cargo` with the musl toolchain

#### Scenario: Previous binary preserved as rollback
- **WHEN** the new binary is transferred to the Droplet
- **THEN** the existing `/usr/local/bin/clawbot` is renamed to `/usr/local/bin/clawbot.prev` before being replaced

#### Scenario: Service restarted after deploy
- **WHEN** the binary is in place
- **THEN** `systemctl restart clawbot` is executed over SSH and the workflow waits for the service to reach `active (running)` state within 30 seconds

#### Scenario: Service fails to start after deploy
- **WHEN** `systemctl status clawbot` shows a failed state within 30 seconds of restart
- **THEN** the workflow fails and logs the service status output for diagnosis (manual rollback required)

---

### Requirement: Secrets management
All sensitive values SHALL be stored as GitHub Actions secrets and injected as environment variables at workflow runtime. No secret SHALL appear in workflow YAML files, build logs, or committed files.

#### Scenario: Secrets injected at deploy time
- **WHEN** the CD workflow SSHes into the Droplet
- **THEN** `BINANCE_API_KEY`, `BINANCE_SECRET`, `TELEGRAM_TOKEN`, `TELEGRAM_ALLOWED_USER_IDS`, `DASHBOARD_TOKEN`, and `TRADING_MODE` are written to the Droplet's systemd environment file from GitHub Secrets

#### Scenario: .env file blocked from commits
- **WHEN** a developer attempts to commit a `.env` file
- **THEN** `.gitignore` prevents it from being staged and a pre-commit hook (if present) blocks the commit

---

### Requirement: Build caching
The CI and CD workflows SHALL cache Cargo registry and compiled dependencies between runs to avoid full rebuilds on every commit.

#### Scenario: Cache hit on unchanged dependencies
- **WHEN** `Cargo.lock` has not changed since the last run
- **THEN** dependency compilation is skipped and the workflow completes faster than a cold build

#### Scenario: Cache invalidated on Cargo.lock change
- **WHEN** a dependency is added or updated (Cargo.lock changes)
- **THEN** the cache is invalidated and dependencies are recompiled from scratch
