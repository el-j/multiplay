//! Embedded mobile web client for Multiplay Remote (TASK-217): transport control, search of the
//! host's library and providers (through the host), pasting a link, and live status of this
//! phone's requests, plus an upload button while the host allows uploads (TASK-218).
//! Self-contained: no external resources, works offline on the LAN, and host-supplied text is
//! only ever set with `textContent`.

#![forbid(unsafe_code)]

pub const MOBILE_WEB_CLIENT_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Multiplay Remote</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0d1117; color: #f0f6fc; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    display: flex; flex-direction: column; align-items: center; min-height: 100vh; padding: 16px;
  }
  .app-container { width: 100%; max-width: 420px; display: flex; flex-direction: column; gap: 16px; }
  .header { display: flex; justify-content: space-between; align-items: center; padding: 8px 4px; }
  .brand { font-size: 1.1rem; font-weight: 800; color: #00eb88; letter-spacing: 1px; }
  .status-pill { font-size: 0.75rem; padding: 4px 8px; border-radius: 12px; background: #238636; color: #fff; }
  .status-pill.disconnected { background: #da3633; }
  .card, .art-card { background: #161b22; border: 1px solid #30363d; border-radius: 16px; padding: 16px; }
  .art-card { padding: 24px 16px; display: flex; flex-direction: column; align-items: center; text-align: center; box-shadow: 0 8px 24px rgba(0,0,0,0.5); }
  .vinyl {
    width: 120px; height: 120px; border-radius: 50%; background: radial-gradient(circle, #00eb88 15%, #1f242c 16%, #0d1117 70%, #30363d 100%);
    box-shadow: 0 4px 16px rgba(0,235,136,0.3); margin-bottom: 16px; animation: spin 8s linear infinite; animation-play-state: paused;
  }
  .vinyl.spinning { animation-play-state: running; }
  @keyframes spin { 100% { transform: rotate(360deg); } }
  .track-title { font-size: 1.2rem; font-weight: 700; margin-bottom: 4px; overflow-wrap: anywhere; }
  .track-artist { font-size: 0.95rem; color: #8b949e; margin-bottom: 16px; overflow-wrap: anywhere; }
  .timeline { width: 100%; display: flex; flex-direction: column; gap: 6px; }
  .time-slider, .volume-slider { width: 100%; -webkit-appearance: none; height: 6px; border-radius: 3px; background: #30363d; outline: none; }
  .time-slider::-webkit-slider-thumb, .volume-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 18px; height: 18px; border-radius: 50%; background: #00eb88; }
  .time-labels { display: flex; justify-content: space-between; font-size: 0.75rem; color: #8b949e; }
  .controls { display: flex; justify-content: center; align-items: center; gap: 20px; margin-top: 12px; }
  .btn { background: #21262d; border: 1px solid #30363d; border-radius: 50%; width: 52px; height: 52px; color: #f0f6fc; font-size: 1.2rem; display: flex; justify-content: center; align-items: center; }
  .btn:active, .btn-action:active { background: #30363d; transform: scale(0.97); }
  .btn-play { width: 68px; height: 68px; font-size: 1.6rem; background: #00eb88; color: #0d1117; border: none; }
  .volume-card { display: flex; align-items: center; gap: 12px; }
  .btn-action { background: #21262d; border: 1px solid #30363d; border-radius: 8px; padding: 10px 14px; color: #c9d1d9; font-size: 0.9rem; font-weight: 600; white-space: nowrap; }
  .btn-action.primary { background: #00eb88; color: #0d1117; border: none; }
  .card-title { font-size: 0.8rem; font-weight: 800; letter-spacing: 1px; color: #00eb88; margin-bottom: 10px; text-transform: uppercase; }
  .search-row { display: flex; gap: 8px; }
  .search-row input { flex: 1; min-width: 0; font-size: 16px; padding: 10px 12px; border-radius: 8px; border: 1px solid #30363d; background: #0d1117; color: #f0f6fc; }
  .notice { font-size: 0.85rem; color: #f0b400; margin-top: 8px; min-height: 1em; overflow-wrap: anywhere; }
  .list { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
  .row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-top: 1px solid #21262d; }
  .row-text { flex: 1; min-width: 0; }
  .row-title { font-size: 0.95rem; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-meta { font-size: 0.75rem; color: #8b949e; }
  .chip { font-size: 0.75rem; padding: 3px 8px; border-radius: 10px; background: #30363d; white-space: nowrap; }
  .chip.Accepted { background: #1f6feb; } .chip.Playing { background: #238636; } .chip.Rejected { background: #6e2a2a; }
  .empty { font-size: 0.85rem; color: #8b949e; }
  .upload-row { margin-top: 10px; }
  .upload-row label { display: inline-block; }
  [hidden] { display: none !important; }
</style>
</head>
<body>
<div class="app-container">
  <div class="header">
    <div class="brand">MULTIPLAY REMOTE</div>
    <div id="status" class="status-pill disconnected">Connecting...</div>
  </div>
  <div class="art-card">
    <div id="vinyl" class="vinyl"></div>
    <div id="title" class="track-title">Not Playing</div>
    <div id="artist" class="track-artist">Multiplay</div>
    <div class="timeline">
      <input type="range" id="seek" class="time-slider" min="0" max="100" value="0" aria-label="Position">
      <div class="time-labels"><span id="elapsed">0:00</span><span id="duration">0:00</span></div>
    </div>
    <div class="controls">
      <button class="btn" id="prev" aria-label="Previous">⏮</button>
      <button class="btn btn-play" id="play" aria-label="Play or pause">▶</button>
      <button class="btn" id="next" aria-label="Next">⏭</button>
    </div>
  </div>
  <div class="card volume-card">
    <span>🔊</span>
    <input type="range" id="volume" class="volume-slider" min="0" max="100" value="85" aria-label="Volume">
    <button class="btn-action" id="vote-skip">Vote skip</button>
  </div>
  <div class="card">
    <div class="card-title">Request a track</div>
    <div class="search-row">
      <input id="query" type="search" enterkeyhint="search" autocomplete="off" placeholder="Search, or paste a YouTube link" aria-label="Search or link">
      <button class="btn-action primary" id="find">Find</button>
    </div>
    <div id="upload-row" class="upload-row" hidden>
      <label class="btn-action" for="file">Upload a song from this phone…</label>
      <input id="file" type="file" accept="audio/*" hidden>
    </div>
    <div id="notice" class="notice"></div>
    <div id="results" class="list"></div>
  </div>
  <div class="card">
    <div class="card-title">My requests</div>
    <div id="mine" class="list"><div class="empty">Nothing requested yet. The host decides what plays.</div></div>
  </div>
</div>
<script>
(function() {
  let ws = null;
  let isPlaying = false;
  let totalSecs = 0;
  const $ = (id) => document.getElementById(id);
  const statusEl = $('status'), vinylEl = $('vinyl'), titleEl = $('title'), artistEl = $('artist');
  const playBtn = $('play'), seekEl = $('seek'), volEl = $('volume');
  const queryEl = $('query'), noticeEl = $('notice'), resultsEl = $('results'), mineEl = $('mine');
  const uploadRow = $('upload-row'), fileEl = $('file');
  const MAX_UPLOAD = 50 * 1024 * 1024;
  const STATUS_TEXT = { Pending: 'Waiting for host', Accepted: 'Accepted', Rejected: 'Declined', Playing: 'Playing now' };
  const LINK = /^https?:\/\//i;
  const sentLabels = [];       // labels of sent requests, answered in order
  const requests = new Map();  // id -> { label, status }

  function fmt(secs) {
    const m = Math.floor(secs / 60), s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
  // Builds an element; text is always set with textContent, never parsed as markup.
  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }
  function notice(text) { noticeEl.textContent = text || ''; }

  // Pairing: the QR code carries a single-use ?token=. The host answers with a
  // long-lived session secret, kept in localStorage for reconnects.
  const SESSION_KEY = 'multiplay.session';
  const params = new URLSearchParams(location.search);
  let pairingToken = params.get('token');
  if (pairingToken) history.replaceState(null, '', location.pathname);
  function loadSession() { try { return localStorage.getItem(SESSION_KEY); } catch (_) { return null; } }
  function saveSession(s) { try { s ? localStorage.setItem(SESSION_KEY, s) : localStorage.removeItem(SESSION_KEY); } catch (_) {} }

  function send(type, payload) {
    if (!ws || ws.readyState !== WebSocket.OPEN) { notice('Not connected to the host.'); return false; }
    const msg = { v: 2, type };
    if (payload !== undefined) msg.payload = payload;
    ws.send(JSON.stringify(msg));
    return true;
  }

  function requestTrack(reference, label) {
    if (send('RequestTrack', reference)) {
      sentLabels.push(label);
      notice('Sent to the host.');
    }
  }

  function renderRequests() {
    mineEl.replaceChildren();
    if (requests.size === 0) {
      mineEl.append(el('div', 'empty', 'Nothing requested yet. The host decides what plays.'));
      return;
    }
    [...requests.entries()].reverse().forEach(([, r]) => {
      const row = el('div', 'row');
      const text = el('div', 'row-text');
      text.append(el('div', 'row-title', r.label));
      row.append(text, el('span', 'chip ' + r.status, STATUS_TEXT[r.status] || r.status));
      mineEl.append(row);
    });
  }

  function renderResults(query, items) {
    resultsEl.replaceChildren();
    notice(items.length ? '' : `Nothing found for “${query}”.`);
    items.forEach((item) => {
      const row = el('div', 'row');
      const text = el('div', 'row-text');
      text.append(el('div', 'row-title', `${item.title} - ${item.artist}`));
      const meta = [item.duration_secs ? fmt(item.duration_secs) : null, item.source].filter(Boolean).join(' · ');
      text.append(el('div', 'row-meta', meta));
      const btn = el('button', 'btn-action', 'Request');
      btn.onclick = () => { requestTrack(item.reference, `${item.title} - ${item.artist}`); btn.disabled = true; };
      row.append(text, btn);
      resultsEl.append(row);
    });
    const asTyped = el('button', 'btn-action', `Ask for “${query}” as typed`);
    asTyped.onclick = () => { requestTrack({ kind: 'Search', value: query }, `“${query}”`); asTyped.disabled = true; };
    resultsEl.append(asTyped);
  }

  function submit() {
    const text = queryEl.value.trim();
    if (!text) return;
    if (LINK.test(text)) {
      requestTrack({ kind: 'Link', value: text }, text);
      queryEl.value = '';
      resultsEl.replaceChildren();
    } else if (send('Search', text)) {
      notice('Searching…');
    }
  }
  function forget(label) {
    const i = sentLabels.indexOf(label);
    if (i >= 0) sentLabels.splice(i, 1);
  }

  // Uploads go to this host only, with this phone's session secret.
  async function uploadFile(file) {
    const session = loadSession();
    if (!session) { notice('Pair this phone again to upload.'); return; }
    if (file.size > MAX_UPLOAD) { notice('That file is larger than 50 MB.'); return; }
    sentLabels.push(file.name);
    notice('Uploading…');
    try {
      const resp = await fetch('/api/upload', {
        method: 'POST',
        headers: { 'Authorization': 'Bearer ' + session, 'X-Filename': encodeURIComponent(file.name) },
        body: file,
      });
      if (resp.ok) { notice('Sent to the host.'); return; }
      forget(file.name);
      notice(`${file.name}: ${await resp.text()}`);
    } catch (_) {
      forget(file.name);
      notice('The upload failed.');
    }
  }
  fileEl.onchange = () => {
    const file = fileEl.files[0];
    fileEl.value = '';
    if (file) uploadFile(file);
  };

  $('find').onclick = submit;
  queryEl.onkeydown = (e) => { if (e.key === 'Enter') submit(); };

  function onMessage(msg) {
    const p = msg.payload;
    if (msg.type === 'SessionGranted') {
      saveSession(p.session);
    } else if (msg.type === 'Status') {
      isPlaying = p.is_playing;
      playBtn.textContent = isPlaying ? '⏸' : '▶';
      vinylEl.classList.toggle('spinning', isPlaying);
      if (p.current_track) {
        titleEl.textContent = p.current_track.title;
        artistEl.textContent = p.current_track.artist;
        totalSecs = p.current_track.duration_secs || 0;
      }
      $('elapsed').textContent = fmt(p.position_secs);
      $('duration').textContent = fmt(totalSecs);
      if (totalSecs > 0) seekEl.value = (p.position_secs / totalSecs) * 100;
      volEl.value = Math.round(p.volume * 100);
    } else if (msg.type === 'RequestUpdate') {
      let r = requests.get(p.id);
      if (!r) { r = { label: sentLabels.shift() || 'Request', status: 'Pending' }; requests.set(p.id, r); }
      r.status = p.status;
      if (p.title) r.label = p.title;
      renderRequests();
    } else if (msg.type === 'RequestRefused') {
      const label = p.search ? null : sentLabels.shift();
      notice(label ? `${label}: ${p.reason}` : p.reason);
    } else if (msg.type === 'Uploads') {
      uploadRow.hidden = !p.enabled;
    } else if (msg.type === 'SearchResults') {
      renderResults(p.query, p.items);
    }
  }

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const session = loadSession();
    let query;
    if (pairingToken) { query = 'token=' + encodeURIComponent(pairingToken); pairingToken = null; }
    else if (session) { query = 'session=' + encodeURIComponent(session); }
    else {
      statusEl.textContent = 'Not paired: scan the QR code in multiplay';
      statusEl.className = 'status-pill disconnected';
      return;
    }
    let opened = false;
    ws = new WebSocket(`${proto}//${location.host}/ws?${query}`);
    ws.onopen = () => {
      opened = true;
      statusEl.textContent = 'Connected';
      statusEl.className = 'status-pill';
      send('GetPlaybackStatus');
    };
    ws.onclose = () => {
      statusEl.className = 'status-pill disconnected';
      if (!opened && query.startsWith('session=')) {
        saveSession(null);
        statusEl.textContent = 'Pairing expired: scan the QR code again';
        return;
      }
      statusEl.textContent = 'Disconnected';
      setTimeout(connect, 2000);
    };
    ws.onmessage = (e) => { try { onMessage(JSON.parse(e.data)); } catch (_) {} };
  }

  playBtn.onclick = () => send(isPlaying ? 'Pause' : 'Play');
  $('prev').onclick = () => send('Previous');
  $('next').onclick = () => send('Next');
  $('vote-skip').onclick = () => send('VoteSkip');
  volEl.oninput = (e) => send('SetVolume', e.target.value / 100.0);
  seekEl.onchange = (e) => {
    if (totalSecs > 0) send('Seek', Math.round((e.target.value / 100.0) * totalSecs));
  };

  connect();
})();
</script>
</body>
</html>
"#;
