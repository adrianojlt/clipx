# Task 18 - Add Lint and Test Steps to CI

**Severity:** High
**Category:** Tooling / CI
**Depends on:** Task 14 (ESLint) and Task 16 (Vitest) and Task 17 (Rust tests) - do those first

## Why This Is a Problem

The current CI pipeline (`.github/workflows/release.yml`) jumps straight from `pnpm install` to building the release binary. There is no step that runs linters or tests. This means:

- A broken JavaScript file that ESLint would catch can be released
- A failing Rust test can be released
- The CI is only a build pipeline, not a quality gate

## Files to Touch

- `.github/workflows/release.yml`

## Exact Change

Find the step that runs `pnpm install` (or frontend dependency installation). After it, and BEFORE the `tauri-action` build step, add:

```yaml
- name: Lint frontend
  run: pnpm lint

- name: Test frontend
  run: pnpm test

- name: Test Rust backend
  run: cargo test --manifest-path src-tauri/Cargo.toml
```

### Where to Insert

The release workflow currently looks roughly like:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: pnpm/action-setup@v4
  - uses: actions/setup-node@v4
  - name: Install Rust stable
    ...
  - name: Install frontend dependencies
    run: pnpm install
  - name: Build the app
    uses: tauri-apps/tauri-action@v0
```

Insert the three new steps between "Install frontend dependencies" and "Build the app":

```yaml
  - name: Install frontend dependencies
    run: pnpm install

  - name: Lint frontend
    run: pnpm lint

  - name: Test frontend
    run: pnpm test

  - name: Test Rust backend
    run: cargo test --manifest-path src-tauri/Cargo.toml

  - name: Build the app
    uses: tauri-apps/tauri-action@v0
```

## Linux-Specific Note

The Rust tests may need display libraries on the Ubuntu runner if they touch clipboard code. For now, limit tests to pure functions (Task 17) that do not require a display. If `cargo test` fails on Ubuntu due to missing `libwebkit2gtk` or similar, add:

```yaml
- name: Test Rust backend
  run: cargo test --manifest-path src-tauri/Cargo.toml
  env:
    DISPLAY: ":0"
```

Or skip display-dependent tests with `#[cfg(not(ci))]` attributes.

## How to Verify

Push a branch with an intentional ESLint error (e.g. an unused variable). The CI run must fail at the lint step, not reach the build step. Fix the error, push again - CI must pass all three new steps and reach the build.
