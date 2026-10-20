> **Beta.** Multiplay is still in early development. Expect rough edges and keep backups of
> playlists you care about. Please report problems at
> https://github.com/el-j/multiplay/issues.

## Download

| System | File |
|---|---|
| macOS 11+ (Apple Silicon and Intel) | `multiplay-{{VERSION}}-macos-universal.dmg` |
| Windows 10/11 (64-bit) | `multiplay-{{VERSION}}-windows-x86_64.zip` |
| Linux (x86_64, glibc 2.35+) | `multiplay-{{VERSION}}-linux-x86_64.tar.gz` |

Each package has the desktop app (`multiplay`) and the terminal version (`multiplay-tui`).

### macOS

Open the `.dmg` and drag **Multiplay** to **Applications**. The beta is not notarized by
Apple yet, so the first launch is blocked: open **System Settings → Privacy & Security** and
click **Open Anyway** next to the Multiplay message. Or, in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Multiplay.app
```

### Windows

Unzip and run `multiplay.exe`. The beta is not code-signed yet, so SmartScreen may warn:
click **More info → Run anyway**.

### Linux

```sh
tar -xzf multiplay-{{VERSION}}-linux-x86_64.tar.gz
./multiplay-{{VERSION}}-linux-x86_64/multiplay
```

Needs ALSA (`libasound2`) and an X11 or Wayland desktop.

### Verify the download

```sh
sha256sum -c SHA256SUMS --ignore-missing   # macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
```
