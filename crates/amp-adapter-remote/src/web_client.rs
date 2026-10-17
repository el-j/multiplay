//! Embedded responsive HTML5/CSS/JS mobile controller for Multiplay Remote.
//! Zero-CDN, fully offline-capable, responsive glassmorphism interface.

#![forbid(unsafe_code)]

pub const MOBILE_WEB_CLIENT_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
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
  .art-card {
    background: #161b22; border: 1px solid #30363d; border-radius: 16px; padding: 24px 16px;
    display: flex; flex-direction: column; align-items: center; text-align: center; box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }
  .vinyl {
    width: 140px; height: 140px; border-radius: 50%; background: radial-gradient(circle, #00eb88 15%, #1f242c 16%, #0d1117 70%, #30363d 100%);
    box-shadow: 0 4px 16px rgba(0,235,136,0.3); margin-bottom: 20px; animation: spin 8s linear infinite; animation-play-state: paused;
  }
  .vinyl.spinning { animation-play-state: running; }
  @keyframes spin { 100% { transform: rotate(360deg); } }
  .track-title { font-size: 1.25rem; font-weight: 700; margin-bottom: 4px; color: #ffffff; }
  .track-artist { font-size: 0.95rem; color: #8b949e; margin-bottom: 16px; }
  .timeline { width: 100%; display: flex; flex-direction: column; gap: 6px; }
  .time-slider { width: 100%; -webkit-appearance: none; height: 6px; border-radius: 3px; background: #30363d; outline: none; }
  .time-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 16px; height: 16px; border-radius: 50%; background: #00eb88; cursor: pointer; }
  .time-labels { display: flex; justify-content: space-between; font-size: 0.75rem; color: #8b949e; }
  .controls { display: flex; justify-content: center; align-items: center; gap: 20px; margin: 12px 0; }
  .btn { background: #21262d; border: 1px solid #30363d; border-radius: 50%; width: 52px; height: 52px; color: #f0f6fc; font-size: 1.2rem; cursor: pointer; display: flex; justify-content: center; align-items: center; }
  .btn:active { background: #30363d; transform: scale(0.96); }
  .btn-play { width: 68px; height: 68px; font-size: 1.6rem; background: #00eb88; color: #0d1117; border: none; box-shadow: 0 4px 16px rgba(0,235,136,0.4); }
  .volume-card { background: #161b22; border: 1px solid #30363d; border-radius: 12px; padding: 12px 16px; display: flex; align-items: center; gap: 12px; }
  .volume-slider { flex: 1; -webkit-appearance: none; height: 6px; border-radius: 3px; background: #30363d; outline: none; }
  .volume-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 16px; height: 16px; border-radius: 50%; background: #58a6ff; cursor: pointer; }
  .jam-bar { display: flex; gap: 8px; justify-content: center; }
  .btn-action { background: #21262d; border: 1px solid #30363d; border-radius: 8px; padding: 10px 16px; color: #c9d1d9; font-size: 0.85rem; font-weight: 600; cursor: pointer; }
  .btn-action:active { background: #30363d; }
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
    <div id="artist" class="track-artist">Multiplay Audiophile</div>
    <div class="timeline">
      <input type="range" id="seek" class="time-slider" min="0" max="100" value="0">
      <div class="time-labels"><span id="elapsed">0:00</span><span id="duration">0:00</span></div>
    </div>
    <div class="controls">
      <button class="btn" id="prev">⏮</button>
      <button class="btn btn-play" id="play">▶</button>
      <button class="btn" id="next">⏭</button>
    </div>
  </div>
  <div class="volume-card">
    <span>🔊</span>
    <input type="range" id="volume" class="volume-slider" min="0" max="100" value="85">
  </div>
  <div class="jam-bar">
    <button class="btn-action" id="vote-skip">🗳️ Vote Skip</button>
  </div>
</div>
<script>
(function() {
  let ws = null;
  let isPlaying = false;
  let totalSecs = 0;
  const statusEl = document.getElementById('status');
  const vinylEl = document.getElementById('vinyl');
  const titleEl = document.getElementById('title');
  const artistEl = document.getElementById('artist');
  const playBtn = document.getElementById('play');
  const seekEl = document.getElementById('seek');
  const volEl = document.getElementById('volume');
  const elapsedEl = document.getElementById('elapsed');
  const durEl = document.getElementById('duration');

  function fmt(secs) {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(`${proto}//${location.host}/ws`);
    ws.onopen = () => {
      statusEl.textContent = 'Connected';
      statusEl.className = 'status-pill';
      ws.send(JSON.stringify({ type: 'GetPlaybackStatus' }));
    };
    ws.onclose = () => {
      statusEl.textContent = 'Disconnected';
      statusEl.className = 'status-pill disconnected';
      setTimeout(connect, 2000);
    };
    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);
        if (msg.type === 'Status') {
          const s = msg.payload;
          isPlaying = s.is_playing;
          playBtn.textContent = isPlaying ? '⏸' : '▶';
          if (isPlaying) vinylEl.classList.add('spinning'); else vinylEl.classList.remove('spinning');
          if (s.current_track) {
            titleEl.textContent = s.current_track.metadata.title;
            artistEl.textContent = s.current_track.metadata.artist;
            if (s.current_track.metadata.duration) {
              totalSecs = s.current_track.metadata.duration.secs || 180;
            }
          }
          elapsedEl.textContent = fmt(s.position_secs);
          durEl.textContent = fmt(totalSecs);
          if (totalSecs > 0) seekEl.value = (s.position_secs / totalSecs) * 100;
          volEl.value = Math.round(s.volume * 100);
        }
      } catch (err) {}
    };
  }

  playBtn.onclick = () => ws && ws.send(JSON.stringify({ type: isPlaying ? 'Pause' : 'Play' }));
  document.getElementById('prev').onclick = () => ws && ws.send(JSON.stringify({ type: 'Previous' }));
  document.getElementById('next').onclick = () => ws && ws.send(JSON.stringify({ type: 'Next' }));
  document.getElementById('vote-skip').onclick = () => ws && ws.send(JSON.stringify({ type: 'VoteSkip' }));
  volEl.oninput = (e) => ws && ws.send(JSON.stringify({ type: 'SetVolume', payload: e.target.value / 100.0 }));
  seekEl.onchange = (e) => {
    if (ws && totalSecs > 0) {
      const target = Math.round((e.target.value / 100.0) * totalSecs);
      ws.send(JSON.stringify({ type: 'Seek', payload: target }));
    }
  };

  connect();
})();
</script>
</body>
</html>
"#;
