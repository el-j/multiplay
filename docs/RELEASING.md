# Releasing

Releases are built by `.github/workflows/release.yml` and published on the
[Releases page](https://github.com/el-j/multiplay/releases).

## Cut a release

1. Make sure `main` is green.
2. The tag must start with the workspace version in `Cargo.toml` (`[workspace.package]`):
   `v0.1.0-beta.1` is accepted for `0.1.0`. Tags containing `-` (beta, rc) are published as
   pre-releases.
3. Tag and push:

   ```sh
   git checkout main && git pull
   git tag v0.1.0-beta.1
   git push origin v0.1.0-beta.1
   ```

The workflow runs the full CI on the tagged commit, builds every package, writes `SHA256SUMS`,
and publishes the release with the notes from `.github/release-notes.md` followed by GitHub's
generated changelog.

**Dry run:** start the workflow by hand (Actions → Release → Run workflow, or
`gh workflow run release.yml --ref <branch>`). It builds everything without publishing; the
packages are attached to the run as `multiplay-<version>-all`.

## What gets built

| File | Contents |
|---|---|
| `multiplay-<v>-macos-universal.dmg` | `Multiplay.app` (Apple Silicon + Intel), `multiplay-tui`, Applications shortcut |
| `multiplay-<v>-windows-x86_64.zip` | `multiplay.exe` (GUI, no console window), `multiplay-tui.exe`; C runtime linked in |
| `multiplay-<v>-linux-{x86_64,aarch64}.AppImage` | Single-file app with desktop entry and icons |
| `multiplay-<v>-linux-{x86_64,aarch64}.tar.gz` | Binaries, `multiplay.desktop`, `multiplay.png` |
| `SHA256SUMS` | Checksums of all of the above |

Linux builds run on Ubuntu 22.04 so they work with glibc 2.35 and newer. The AppImage tools
are pinned by version and checksum in the workflow.

## Code signing (optional)

Without signing secrets the workflow still publishes: macOS gets an ad-hoc signature, Windows
is unsigned, and the release notes explain the one-time first-launch step. Once the secrets
below are set (Settings → Secrets and variables → Actions), the next release is signed and the
workaround text is dropped from its notes automatically.

### macOS: Developer ID and notarization

Needs a paid [Apple Developer Program](https://developer.apple.com/programs/) membership.

1. In Xcode or the developer portal, create a **Developer ID Application** certificate and
   export it with its private key from Keychain Access as a `.p12` with a password.
2. Create an [app-specific password](https://support.apple.com/102654) for your Apple ID.
3. Add these secrets:

| Secret | Value |
|---|---|
| `MACOS_CERTIFICATE` | `base64 -i DeveloperID.p12` output |
| `MACOS_CERTIFICATE_PASSWORD` | the `.p12` export password |
| `APPLE_ID` | Apple ID e-mail of the developer account |
| `APPLE_TEAM_ID` | 10-character team ID (developer portal → Membership) |
| `APPLE_APP_PASSWORD` | the app-specific password |

With only the first two, builds are Developer ID signed (hardened runtime) but not notarized.
With all five, the disk image is notarized and stapled, so it opens without any warning.

### Windows: Azure Artifact Signing

[Azure Artifact Signing](https://learn.microsoft.com/azure/artifact-signing/) (formerly
Trusted Signing) is the low-cost way to sign since certificates must live in hardware. Set up
an account and a certificate profile, create an app registration with the
"Artifact Signing Certificate Profile Signer" role, then add:

| Secret | Value |
|---|---|
| `AZURE_TENANT_ID` | directory (tenant) ID |
| `AZURE_CLIENT_ID` | app registration (client) ID |
| `AZURE_CLIENT_SECRET` | client secret of that app registration |
| `AZURE_SIGNING_ENDPOINT` | account endpoint, e.g. `https://weu.codesigning.azure.net/` |
| `AZURE_SIGNING_ACCOUNT` | signing account name |
| `AZURE_CERTIFICATE_PROFILE` | certificate profile name |

Signing is switched on by `AZURE_CLIENT_ID`. SmartScreen reputation still builds up over the
first downloads of a newly signed app.

## App icon

The artwork is drawn in code. After changing it, regenerate every size and the `.icns`/`.ico`
and commit the result:

```sh
cargo run -p amp-adapter-gui --example make_icons
```

`build.rs` embeds the icon into the Windows `.exe`; the window icon and the macOS bundle icon
come from the same files.
