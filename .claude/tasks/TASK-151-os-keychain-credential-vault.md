# TASK-151: OS Keychain Credential Vault (with Encrypted-File Fallback)

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `security`
- Priority: **P1**
- Audit findings: `SEC-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-147

## Objectives
1. `CredentialVaultPort` backed by the `keyring` crate; fallback to an AEAD-encrypted file (`chacha20poly1305`) for headless Linux.
2. Return `SecretString` (not `String`) from `get_secret`; never log secrets.

## Acceptance Criteria (user-visible / measurable)
- Soulseek password survives restart and is not present in any plaintext file.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/keyring.rs`
- `crates/amp-ports/src/driven.rs`

## Verification Command
`cargo test -p amp-adapter-storage keyring`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
