//! Port contract checks (TASK-187): what every implementation of a port has to do, written once
//! and run against each adapter (and each fake used in tests), so the engine can rely on it.

#![forbid(unsafe_code)]

use amp_ports::{LibraryStoragePort, PcmStreamPort};
use std::time::Duration;

/// Checks a decoder against the `PcmStreamPort` contract. `open` makes a fresh decoder.
///
/// - The sample rate is a real audio rate.
/// - Reads return whole stereo pairs, even into an odd-sized buffer, and only finite samples.
/// - `Ok(0)` is the end and stays the end.
/// - `elapsed` never goes backwards and matches the frames read; `duration`, when known, matches
///   the decoded length.
/// - A seekable decoder lands near the target, and seeking back to 0 after the end plays again.
///
/// # Panics
/// With a message naming `label` and the broken rule.
pub fn check_pcm_stream(label: &str, open: &dyn Fn() -> Box<dyn PcmStreamPort>) {
    let mut dec = open();
    let rate = dec.sample_rate();
    assert!(
        (8_000..=768_000).contains(&rate),
        "{label}: sample rate {rate}"
    );

    // An odd buffer: a decoder must never split a stereo pair.
    let mut buf = vec![0.0f32; 1_001];
    let mut frames: u64 = 0;
    let mut last_elapsed = Duration::ZERO;
    for _ in 0..1_000_000 {
        let n = dec
            .read_interleaved(&mut buf)
            .unwrap_or_else(|e| panic!("{label}: read failed: {e}"));
        if n == 0 {
            break;
        }
        assert!(
            n <= buf.len() && n.is_multiple_of(2),
            "{label}: read {n} samples into {}",
            buf.len()
        );
        assert!(
            buf[..n].iter().all(|s| s.is_finite()),
            "{label}: non-finite samples"
        );
        frames += n as u64 / 2;
        let elapsed = dec.elapsed();
        assert!(
            elapsed >= last_elapsed,
            "{label}: elapsed went back from {last_elapsed:?} to {elapsed:?}"
        );
        last_elapsed = elapsed;
    }
    assert!(frames > 0, "{label}: no audio at all");
    for _ in 0..3 {
        assert_eq!(
            dec.read_interleaved(&mut buf).ok(),
            Some(0),
            "{label}: the end must stay the end"
        );
    }

    let decoded = Duration::from_secs_f64(frames as f64 / f64::from(rate));
    let tolerance = |d: Duration| d.mul_f64(0.02).max(Duration::from_millis(30));
    assert!(
        last_elapsed.abs_diff(decoded) <= tolerance(decoded),
        "{label}: elapsed {last_elapsed:?} at the end, but {decoded:?} decoded"
    );
    if let Some(total) = dec.duration() {
        assert!(
            total.abs_diff(decoded) <= tolerance(decoded),
            "{label}: duration {total:?}, decoded {decoded:?}"
        );
    }

    // Seeking (optional for a stream, but if it says Ok it has to mean it).
    let mut dec = open();
    let target = decoded / 2;
    if let Ok(landed) = dec.seek(target) {
        assert!(
            landed.abs_diff(target) <= Duration::from_millis(100),
            "{label}: seek to {target:?} landed at {landed:?}"
        );
        let n = dec.read_interleaved(&mut buf).unwrap_or_default();
        assert!(n > 0, "{label}: nothing after a seek to the middle");
        assert!(
            dec.elapsed() >= landed,
            "{label}: elapsed before the seek point after seeking"
        );
        while dec.read_interleaved(&mut buf).unwrap_or(0) > 0 {}
        dec.seek(Duration::ZERO)
            .unwrap_or_else(|e| panic!("{label}: seek back to 0 failed: {e}"));
        assert!(
            dec.read_interleaved(&mut buf).unwrap_or(0) > 0,
            "{label}: no audio after seeking back to 0"
        );
    }
}

/// Checks a library store against the `LibraryStoragePort` contract: what is stored comes back,
/// storing again replaces, removed tracks are gone, and search finds by title and artist.
///
/// # Panics
/// With a message naming `label` and the broken rule.
pub async fn check_library_storage(label: &str, store: &dyn LibraryStoragePort) {
    use amp_domain::{AudioSource, Track, TrackMetadata};
    let track = |title: &str, artist: &str, path: &str| {
        Track::with_stable_id(
            AudioSource::LocalFile { path: path.into() },
            TrackMetadata::new(title, artist),
        )
    };
    let a = track("Windowlicker", "Aphex Twin", "/m/a.flac");
    let b = track("Roygbiv", "Boards of Canada", "/m/b.flac");
    store
        .store_track(&a)
        .await
        .unwrap_or_else(|e| panic!("{label}: store: {e}"));
    store
        .store_track(&b)
        .await
        .unwrap_or_else(|e| panic!("{label}: store: {e}"));
    let all = store
        .fetch_all()
        .await
        .unwrap_or_else(|e| panic!("{label}: fetch: {e}"));
    assert_eq!(all.len(), 2, "{label}: stored two");

    let mut renamed = a.clone();
    renamed.metadata.title = "Windowlicker (edit)".into();
    store
        .store_track(&renamed)
        .await
        .unwrap_or_else(|e| panic!("{label}: store again: {e}"));
    let all = store
        .fetch_all()
        .await
        .unwrap_or_else(|e| panic!("{label}: fetch: {e}"));
    assert_eq!(all.len(), 2, "{label}: storing again replaces");
    assert!(
        all.iter()
            .any(|t| t.metadata.title == "Windowlicker (edit)"),
        "{label}: the new version is kept"
    );

    let found = store
        .search_cached("boards")
        .await
        .unwrap_or_else(|e| panic!("{label}: search: {e}"));
    assert!(
        found.iter().any(|t| t.id == b.id),
        "{label}: search finds by artist"
    );
    let found = store
        .search_cached("window")
        .await
        .unwrap_or_else(|e| panic!("{label}: search: {e}"));
    assert!(
        found.iter().any(|t| t.id == a.id),
        "{label}: search finds by title"
    );

    store
        .remove_track(&b.id)
        .await
        .unwrap_or_else(|e| panic!("{label}: remove: {e}"));
    let all = store
        .fetch_all()
        .await
        .unwrap_or_else(|e| panic!("{label}: fetch: {e}"));
    assert_eq!(all.len(), 1, "{label}: removed tracks are gone");
}
