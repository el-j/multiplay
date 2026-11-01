> **Beta.** Multiplay is still in early development. Expect rough edges and keep backups of
> playlists you care about. Please report problems at
> https://github.com/el-j/multiplay/issues.

## Download

| System | File |
|---|---|
| macOS 11+ (Apple Silicon and Intel) | `multiplay-{{VERSION}}-macos-universal.dmg` |
| Windows 10/11 (64-bit) | `multiplay-{{VERSION}}-windows-x86_64.msi` (installer) or `.zip` |
| Linux x86_64 | `multiplay-{{VERSION}}-linux-x86_64.deb` (Debian, Ubuntu), `.AppImage` or `.tar.gz` |
| Linux ARM64 (e.g. Raspberry Pi 4/5 with a 64-bit OS) | `multiplay-{{VERSION}}-linux-aarch64.deb`, `.AppImage` or `.tar.gz` |

Each package has the desktop app (`multiplay`) and the terminal version (`multiplay-tui`), plus
`CHANGELOG.md`, `privacy.md` (every connection the app makes) and `THIRD_PARTY_LICENSES.html`.

### macOS

Open the `.dmg` and drag **Multiplay** to **Applications**.
<!-- unsigned-macos -->

This beta is not notarized by Apple yet, so macOS blocks the first launch. Open
**System Settings → Privacy & Security** and click **Open Anyway** next to the Multiplay
message. Or, in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Multiplay.app
```
<!-- /unsigned-macos -->

### Windows

Run the `.msi` installer: Multiplay goes to Program Files with a Start menu entry, and later
versions install over it. Or unzip the `.zip` and run `multiplay.exe`.
<!-- unsigned-windows -->

This beta is not code-signed yet, so SmartScreen may warn: click **More info → Run anyway**.
<!-- /unsigned-windows -->

### Linux

On Debian, Ubuntu and their relatives the `.deb` adds Multiplay to the app menu:

```sh
sudo apt install ./multiplay-{{VERSION}}-linux-x86_64.deb
```

The AppImage runs on most distributions (glibc 2.35 or newer, e.g. Ubuntu 22.04+, Debian 12+,
Fedora 36+):

```sh
chmod +x multiplay-{{VERSION}}-linux-x86_64.AppImage
./multiplay-{{VERSION}}-linux-x86_64.AppImage
```

If it does not start because FUSE is missing, add `--appimage-extract-and-run`. The tarball
has the plain binaries plus `multiplay.desktop` and `multiplay.png` for a menu entry. Both need
ALSA (`libasound2`) and an X11 or Wayland desktop.

### Verify the download

```sh
sha256sum -c SHA256SUMS --ignore-missing   # macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
```
