# AUDIT-11 — Legal, Licensing & Product Readiness

Part of [AUDIT-2026-10-17](README.md).

> This is an engineering risk assessment, not legal advice. Have a lawyer review it before public distribution, especially the provider integrations.

## LEG-01 — No README, LICENSE or user documentation · P0 for any public release
**Evidence:** `git ls-files` contains neither `README*` nor `LICENSE*`. Without a license the code is "all rights reserved" by default: nobody can legally redistribute or contribute. There are no user docs, FAQ or supported-features page.
**Fix:** Choose a license (GPL-3.0 or MPL-2.0 suit a player that builds on community protocols; MIT/Apache-2.0 if maximum adoption matters). Write a README with an honest feature matrix (stable / experimental / planned), plus CONTRIBUTING and a CODE_OF_CONDUCT. → TASK-114

## LEG-02 — Third-party license obligations unmet · P1
**Evidence:** Release artifacts ship no dependency license texts (MIT/Apache/BSD/MPL/ISC require notices). eframe ships fonts with their own licenses.
**Fix:** Generate `THIRD_PARTY_LICENSES.html` with `cargo-about` in the release job, link it from the About dialog, and enforce allowed licenses with `cargo-deny`. → TASK-182

## LEG-03 — Provider Terms of Service exposure · P1
| Provider | Risk | Notes |
| :--- | :--- | :--- |
| YouTube | Medium–High | ToS prohibit downloading and non-official clients. Streaming-only players (NewPipe, FreeTube) exist, but downloading is the higher-risk feature. App stores reject YouTube downloaders. |
| Spotify (librespot) | High | Violates the Developer Terms; requires Premium; accounts have been banned. |
| SoundCloud | Medium | Undocumented API, scraped `client_id`; Go+ content excluded. |
| Bandcamp | Low–Medium | No public API; scraping public pages for preview streams. Respect artists, and link to purchase pages. |
| Soulseek | Medium | Protocol is open to third-party clients (Nicotine+ etc.), but users share copyrighted files at their own risk. Show a clear disclaimer, and respect the network's sharing etiquette. |
**Fix:** Every gray-area provider is **opt-in**, with an explicit in-app disclosure. Downloading is off by default and labelled "for content you have rights to". Bandcamp results always link to the purchase page. Keep provider adapters in separately buildable crates (Cargo features) so distributions can omit them. → TASK-192

## LEG-04 — No privacy statement · P2
The app connects to YouTube, HTTP hosts, (future) Soulseek servers and peers, and exposes a LAN server. Users deserve a page listing every network connection and what is sent. No telemetry without opt-in. → TASK-193

## LEG-05 — Trademark use: "Winamp" · P1
See UX-10. Use nominative wording ("supports classic Winamp® skins") and never brand windows or the product with it. Bundled skins must be original or permissively licensed; shipping the default Winamp "base skin" bitmaps would be infringing. → TASK-194

## LEG-06 — Product positioning vs. reality · P1
The project's own docs advertise "zero C linkage", "bit-perfect", "EBU R128", "librespot", "libp2p Noise/Yamux" and "35 MB footprint". Public claims that the software doesn't meet erode trust with exactly the audiophile and privacy-minded audience this product targets. Every public-facing claim must be backed by a test or a measurement. → TASK-114, TASK-115
