# Privacy

multiplay is a local music player. It has **no accounts, no ads and no tracking**.

- **No telemetry:** no usage statistics, crash reports or analytics are collected or sent.
- **No update checks unless you ask:** checking for new versions is off until you switch it on
  in Preferences → About; then it asks GitHub for the release list at most once a day.
- **No background connections:** nothing goes online until you do one of the things listed below.

This page lists every connection the app can make, what is sent, and when. It is checked against
the code by a test (`crates/amp-app/tests/privacy_statement.rs`): a new hard-coded server address
fails the build until it is listed here.

## Connections multiplay makes

| When | Server | What is sent |
|---|---|---|
| You search in the search window, or a paired phone searches, with YouTube on | `www.youtube.com` | Your search text |
| The same search, with Bandcamp on | `bandcamp.com` (`/api/bcsearch_public_api/…`) | Your search text |
| You play or add a YouTube result or link | `www.youtube.com`, `*.googlevideo.com` | The video ID; the audio is then downloaded from Google's video servers |
| You play or add a Bandcamp result or link | `<artist>.bandcamp.com`, `*.bcbits.com` | The release page address; the audio stream is downloaded from Bandcamp's media servers |
| You play an internet radio station or a stream link | The server in that link | A normal HTTP request for the stream. `Icy-MetaData: 1` asks the station to send the song titles |
| You click a link in Preferences → About | `github.com` | Your web browser opens the project page or this statement |
| You switched on "Check for new versions" (off by default) | `api.github.com` | A request for multiplay's release list, at most once a day |

A search goes only to the online sources switched on in Preferences → Accounts (or in the
first-run setup): YouTube and Bandcamp are on unless you switch them off. With both off, nothing
is sent. Your own library is always searched on this computer only.

Every one of these is a plain request from your computer to that service, the same as visiting it in
a browser. The service sees your IP address and what you asked for, and its own privacy policy
applies. multiplay does not send them anything else: no identifier, no cookies from other sites, and
no information about your library.

Links pasted into the search box (`youtu.be/…`, `youtube.com/watch?…`, `*.bandcamp.com/…`, stream
URLs) are only fetched when you paste them.

### User-Agent

Requests made by multiplay's own HTTP client (Bandcamp, radio streams, stream links) say who is
asking:

```
User-Agent: multiplay/<version> (+https://github.com/el-j/multiplay)
```

YouTube requests are made by the [rusty_ytdl](https://crates.io/crates/rusty_ytdl) library, which
sends its own browser-style headers.

## Remote control (off by default)

Preferences → Remote Control lets a phone control playback.

- **Off by default.** While it is off, multiplay does not listen on any network port.
- **This computer only by default.** When you turn it on, it listens on `127.0.0.1` port `9002`, so
  only programs on this computer can connect.
- **Your network only if you allow it.** "Allow devices on my network" listens on all network
  interfaces, so phones on the same Wi-Fi can connect after pairing with the QR code. Nothing is
  exposed to the internet unless your router forwards that port.
- **Pairing:** paired devices are remembered in `~/.multiplay/remote_sessions.json`. Only a hash of
  each device's secret is stored. The device name shown in the Remote window is taken from the
  phone browser's User-Agent.
- **Phone uploads are off by default.** If you turn them on, uploaded audio files are kept in
  `~/.multiplay/remote-uploads/` and deleted when remote control stops.

## Soulseek (off by default)

"Use Soulseek" in Preferences → P2P lets you search and play music other Soulseek users share.
multiplay does not speak the Soulseek protocol itself: it talks to **slskd**, a separate,
open-source Soulseek client, over its API.

- **Off by default.** While it is off, multiplay contacts no Soulseek service and runs no slskd.
- **Your own slskd:** multiplay connects to the address you enter (normally on this computer)
  with the API key you enter.
- **"Install and run slskd for me":** only when you click Install, multiplay downloads slskd (version
  0.26.2) from `github.com/slskd/slskd` (checked against a fixed SHA-256) into `~/.multiplay/slskd/`, and runs
  it while multiplay runs: listening on `127.0.0.1` only, sharing **no** folders.
- **What slskd sends:** slskd logs in to the Soulseek network (`server.slsknet.org`) with your
  account, sends your searches there, and downloads files you play directly from other users,
  who see your user name and IP address. Downloads go to `~/.multiplay/slskd/downloads/`.
- Your Soulseek password and the slskd API key are kept in the encrypted vault
  (`~/.multiplay/vault.bin`), not in `config.toml`, and are passed to slskd through its
  environment, not its command line.

## Private mesh (off by default)

"Join a private mesh" in Preferences → P2P connects your own devices and friends' devices that
share a **mesh code**, so each can search and play the music the others share.

- **Off by default.** While it is off, multiplay opens no mesh port and sends no announcements.
- **Only devices with the code connect.** The code is a 256-bit key; a device without it is
  refused during the first handshake, before any data. Everything between members is encrypted
  (Noise). The code and this device's identity are kept in the encrypted vault
  (`~/.multiplay/vault.bin`), not in `config.toml`.
- **On your local network** multiplay announces itself with mDNS (service `_multiplay._tcp.local.`) and listens
  on TCP port 4040. The announcement carries this device's peer id and a hash that identifies the
  mesh, never the code. There is no internet server: members on other networks are not found yet.
- **What members see:** your peer id and IP address, the title, artist, album and length of the
  files you share, and those files' contents when they play them. Nothing is shared until you tick
  "Share my music folders"; then only audio files in your music folders are offered, looked up by
  an id, never by path.
- **What you send:** your searches go to every connected member.

## Media keys and Now Playing

multiplay tells the operating system what is playing (title, artist, album, length, and the cover
image address for online tracks) so media keys and the system's Now Playing panel work: Control
Center on macOS, the media overlay on Windows, MPRIS on Linux (over the local D-Bus session, which
other programs on this computer, such as `playerctl`, can read). This stays on your computer.

## Files multiplay stores on your computer

| Location | Contents |
|---|---|
| `~/.multiplay/` | Session playlist, library database, paired remote devices, phone uploads |
| `~/.multiplay/logs/` | Log files (one per day, the last 7 kept), crash reports and diagnostics zips you save. Track titles and stream addresses appear in the logs |
| `~/.config/multiplay/` | `config.toml` (settings) and the window layout |
| `multiplay mixes/` beside your first music folder (else `~/.multiplay/mixes/`) | DJ mixes you record with REC, as FLAC files |

Nothing in these folders is uploaded anywhere. Delete them to reset multiplay.

**Diagnostics** (Preferences → About) are only created when you click "Copy diagnostics" or "Save
diagnostics". They contain the version, your system, your settings with passwords and tokens
removed, the end of the newest log and the latest crash report. You decide whether to share them.
