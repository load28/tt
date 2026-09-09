# TASK-313: Resolve language services through linked TypeScript packages

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-02
- **Commit**: `TASK-313: resolve linked TypeScript toolchains`

## Purpose

Restore editor type inference when a consumer installs the repository's local
TypeScript package with a `file:` dependency.

## Scope

- Included: TypeScript language-service executable resolution and regression
  coverage for linked packages
- Excluded: Changes to TypeScript packaging, content mapper configuration, and
  editor presentation

## Decisions

### Decision 1: Resolve the platform package from the client package's real location

- **Context**: A `file:` dependency can link only the `typescript` client package
  into the consumer while its platform package remains beside the link target.
- **Alternatives considered**: Copy the platform package into each demo, configure
  an editor-only compiler path, or teach the shared toolchain resolver about npm
  package links.
- **Decision and rationale**: Follow the installed client package to its canonical
  package root and resolve the platform sibling there. This matches Node package
  identity and keeps CLI and editor consumers on the same toolchain.

## Work log

- 2026-09-02: Reproduced hover failure in `ttx-enterprise-ops`; the client API was
  linked from the tt checkout while `service_binary` searched only the consumer's
  physical `node_modules`.
- 2026-09-02: Added canonical package-root resolution and a linked-client
  regression test, rebuilt the release compiler, and verified live demo hover
  responses.
- 2026-09-02: Ran `./scripts/ci rust`; formatting, clippy, unit, integration,
  snapshot, and documentation tests passed.

## Issues and resolutions

### Issue 1: Linked TypeScript client has no consumer-local platform sibling

- **Symptom**: Every engine `hover` request returned `no TypeScript language
  server found` although type checking could load the TypeScript API.
- **Cause**: `service_binary` did not follow the client package symlink before
  locating `@typescript/typescript-<platform>-<arch>`.
- **Resolution**: `service_binary` now checks the ordinary consumer install and
  then the canonical client package's containing `node_modules` using the same
  distribution layout contract.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci rust`
- [x] Live `ttx-enterprise-ops` engine hover probes for inferred callback and
  reducer state types

## Result

`src/typescript/toolchain.rs` now resolves the native language-service binary
for ordinary, linked, and package-store TypeScript client layouts. The local
enterprise demo returns `Metric`, `Service`, `Approval`, and `DashboardState`
for the previously unavailable hover queries.
