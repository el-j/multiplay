//! Renders the Multiplay app icon into `assets/icon`: one PNG per size plus the macOS `.icns`
//! and Windows `.ico`. Run after changing the artwork:
//!
//! ```sh
//! cargo run -p amp-adapter-gui --example make_icons
//! ```
//!
//! Every size is drawn directly from signed distance fields (no downscaling), so small icons
//! stay sharp. Colours follow the window chrome: navy strip, gold grip.

#![forbid(unsafe_code)]

use amp_adapter_gui::app_icon;
use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

type V = (f32, f32);

fn sub(a: V, b: V) -> V {
    (a.0 - b.0, a.1 - b.1)
}
fn dot(a: V, b: V) -> f32 {
    a.0 * b.0 + a.1 * b.1
}
fn len(a: V) -> f32 {
    dot(a, a).sqrt()
}
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}
fn over(dst: [f32; 3], src: [f32; 3], alpha: f32) -> [f32; 3] {
    mix(dst, src, alpha.clamp(0.0, 1.0))
}

/// Superellipse ("squircle") of half-size `h`; approximate distance, negative inside.
fn sd_squircle(p: V, h: f32) -> f32 {
    let n = 5.0;
    let (x, y) = ((p.0 - 0.5).abs() / h, (p.1 - 0.5).abs() / h);
    ((x.powf(n) + y.powf(n)).powf(1.0 / n) - 1.0) * h
}

/// Exact triangle distance (Inigo Quilez), negative inside.
fn sd_triangle(p: V, a: V, b: V, c: V) -> f32 {
    let (e0, e1, e2) = (sub(b, a), sub(c, b), sub(a, c));
    let (v0, v1, v2) = (sub(p, a), sub(p, b), sub(p, c));
    let proj = |v: V, e: V| {
        let t = (dot(v, e) / dot(e, e)).clamp(0.0, 1.0);
        sub(v, (e.0 * t, e.1 * t))
    };
    let (q0, q1, q2) = (proj(v0, e0), proj(v1, e1), proj(v2, e2));
    let s = (e0.0 * e2.1 - e0.1 * e2.0).signum();
    let pick = |q: V, v: V, e: V| (dot(q, q), s * (v.0 * e.1 - v.1 * e.0));
    let d = [pick(q0, v0, e0), pick(q1, v1, e1), pick(q2, v2, e2)];
    let dist = d.iter().map(|x| x.0).fold(f32::MAX, f32::min);
    let side = d.iter().map(|x| x.1).fold(f32::MAX, f32::min);
    -dist.sqrt() * side.signum()
}

/// Arc of radius `r` and stroke width `w` around `c`, opening to the right by ±`span` radians,
/// with round caps.
fn sd_arc(p: V, c: V, r: f32, w: f32, span: f32) -> f32 {
    let d = sub(p, c);
    if d.1.atan2(d.0).abs() <= span {
        (len(d) - r).abs() - w / 2.0
    } else {
        let cap = |sgn: f32| len(sub(d, (r * span.cos(), sgn * r * span.sin())));
        cap(1.0).min(cap(-1.0)) - w / 2.0
    }
}

fn render(size: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let px = 1.0 / size as f32;
    // Coverage of a shape from its distance: a one-pixel anti-aliased edge.
    let cover = |d: f32| (0.5 - d / px).clamp(0.0, 1.0);
    // Small icons fill more of the canvas so they stay readable in a taskbar.
    let half = if size <= 48 { 0.47 } else { 0.41 };
    let k = half / 0.41;
    let at = |x: f32, y: f32| (0.5 + (x - 0.5) * k, 0.5 + (y - 0.5) * k);
    let tri = (at(0.31, 0.30), at(0.31, 0.70), at(0.60, 0.50));
    let centre = at(0.50, 0.50);
    let rim = (0.012 * k).max(px * 0.9);

    ImageBuffer::from_fn(size, size, |x, y| {
        let p = ((x as f32 + 0.5) * px, (y as f32 + 0.5) * px);
        let body = sd_squircle(p, half);
        let alpha = cover(body);
        if alpha <= 0.0 {
            return Rgba([0, 0, 0, 0]);
        }
        let t = ((p.1 - (0.5 - half)) / (2.0 * half)).clamp(0.0, 1.0);
        let mut c = mix([60.0, 72.0, 116.0], [20.0, 23.0, 38.0], t);
        // Soft highlight across the top.
        c = over(c, [120.0, 136.0, 190.0], 0.18 * (1.0 - t * 2.5).max(0.0));
        // Gold rim just inside the edge.
        c = over(
            c,
            [214.0, 170.0, 72.0],
            0.55 * cover(body.abs() - rim / 2.0 - px * 0.5),
        );

        let waves = [(0.17, 1.0), (0.28, 0.62)];
        for (r, a) in waves {
            let d = sd_arc(p, centre, r * k, 0.05 * k, 0.75);
            c = over(c, [238.0, 240.0, 246.0], a * cover(d));
        }
        let rounding = 0.035 * k;
        let d_tri = sd_triangle(p, tri.0, tri.1, tri.2) - rounding;
        let shadow = sd_triangle(sub(p, (0.0, 0.02 * k)), tri.0, tri.1, tri.2) - rounding;
        c = over(
            c,
            [8.0, 10.0, 18.0],
            0.45 * (0.5 - shadow / (0.04 * k)).clamp(0.0, 1.0),
        );
        let g = ((p.1 - tri.0 .1) / (tri.1 .1 - tri.0 .1)).clamp(0.0, 1.0);
        let gold = mix([250.0, 212.0, 118.0], [206.0, 150.0, 50.0], g);
        c = over(c, gold, cover(d_tri));

        let [r, g, b] = c.map(|v| v.round().clamp(0.0, 255.0) as u8);
        Rgba([r, g, b, (alpha * 255.0).round() as u8])
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon");
    std::fs::create_dir_all(&dir)?;
    for size in app_icon::SIZES {
        let path = dir.join(format!("multiplay-{size}.png"));
        render(size).save(&path)?;
        println!("wrote {}", path.display());
    }
    std::fs::write(dir.join("multiplay.ico"), app_icon::build_ico(&dir)?)?;
    std::fs::write(dir.join("multiplay.icns"), app_icon::build_icns(&dir)?)?;
    println!("wrote multiplay.ico and multiplay.icns");
    Ok(())
}
