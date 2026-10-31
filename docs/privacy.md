# Privacy

multiplay is a local music player. It has **no accounts, no ads and no tracking**.

- **No telemetry:** no usage statistics, crash reports or analytics are collected or sent.
- **No update checks:** multiplay never phones home to ask for a newer version.
- **No background connections:** nothing goes online until you do one of the things listed below.

This page lists every connection the app can make, what is sent, and when. It is checked against
the code by a test (`crates/amp-app/tests/privacy_statement.rs`): a new hard-coded server address
fails the build until it is listed here.

## Connections multiplay makes

| When | Server | What is sent |
|---|---|---|
| You search in the search window, or a paired phone searches | `www.youtube.com` | Your search text |
| The same search | `bandcamp.com` (`/api/bcsearch_public_api/…`) | Your search text |
| You play or add a YouTube result or link | `www.youtube.com`, `*.googlevideo.com` | The video ID; the audio is then downloaded from Google's video servers |
| You play or add a Bandcamp result or link | `<artist>.bandcamp.com`, `*.bcbits.com` | The release page address; the audio stream is downloaded from Bandcamp's media servers |
| You play an internet radio station or a stream link | The server in that link | A normal HTTP request for the stream. `Icy-MetaData: 1` asks the station to send the song titles |
| You click a link in Preferences → About | `github.com` | Your web browser opens the project page or this statement |

A search is always sent to both YouTube and Bandcamp; your own library is searched on this
computer only.

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

## Not connected yet

The code contains a Soulseek client (`server.slsknet.org:2242`) and a peer-to-peer sharing mesh. In
this beta neither makes any connection: there is no Soulseek login, and the mesh only searches what
is already on this computer. This page will be updated before either goes online.

## Files multiplay stores on your computer

| Location | Contents |
|---|---|
| `~/.multiplay/` | Session playlist, library database, paired remote devices, phone uploads |
| `~/.multiplay/logs/` | Log files (one per day, the last 7 kept), crash reports and diagnostics zips you save. Track titles and stream addresses appear in the logs |
| `~/.config/multiplay/` | `config.toml` (settings) and the window layout |

Nothing in these folders is uploaded anywhere. Delete them to reset multiplay.

**Diagnostics** (Preferences → About) are only created when you click "Copy diagnostics" or "Save
diagnostics". They contain the version, your system, your settings with passwords and tokens
removed, the end of the newest log and the latest crash report. You decide whether to share them.
