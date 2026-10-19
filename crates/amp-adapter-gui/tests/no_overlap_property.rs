//! Property: whatever the user does with the windows (drag, resize, shade, show, double size),
//! no two visible windows ever overlap.

#![forbid(unsafe_code)]

use amp_adapter_gui::chrome::drag::DragSession;
use amp_adapter_gui::chrome::layout::TITLE_HEIGHT;
use amp_adapter_gui::chrome::resize::min_inner_size;
use amp_adapter_gui::docking::{
    clamp_growth, overlaps, rescale_layout, resize_with_attached, separate_from,
};
use amp_adapter_gui::{DockRect, WindowId};
use proptest::prelude::*;

const IDS: [WindowId; 4] = [
    WindowId::Main,
    WindowId::Equalizer,
    WindowId::Playlist,
    WindowId::DjConsole,
];

#[derive(Debug, Clone)]
enum Op {
    Drag(usize, f32, f32),
    Resize(usize, f32, f32),
    Shade(usize),
    Toggle(usize),
    DoubleSize,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..4usize, -600.0f32..600.0, -600.0f32..600.0).prop_map(|(i, x, y)| Op::Drag(i, x, y)),
        (0..4usize, 0.0f32..900.0, 0.0f32..900.0).prop_map(|(i, w, h)| Op::Resize(i, w, h)),
        (0..4usize).prop_map(Op::Shade),
        (1..4usize).prop_map(Op::Toggle),
        Just(Op::DoubleSize),
    ]
}

struct World {
    rects: Vec<(WindowId, DockRect)>,
    visible: [bool; 4],
    shaded: [Option<f32>; 4],
    zoom: f32,
}

impl World {
    fn new() -> Self {
        Self {
            rects: vec![
                (WindowId::Main, DockRect::new(120.0, 100.0, 480.0, 210.0)),
                (
                    WindowId::Equalizer,
                    DockRect::new(120.0, 310.0, 480.0, 200.0),
                ),
                (
                    WindowId::Playlist,
                    DockRect::new(120.0, 510.0, 480.0, 300.0),
                ),
                (
                    WindowId::DjConsole,
                    DockRect::new(610.0, 530.0, 480.0, 220.0),
                ),
            ],
            visible: [true, true, true, false],
            shaded: [None; 4],
            zoom: 1.0,
        }
    }

    fn layout(&self) -> Vec<(WindowId, DockRect)> {
        self.rects
            .iter()
            .enumerate()
            .filter(|(i, _)| self.visible[*i])
            .map(|(_, r)| *r)
            .collect()
    }

    fn set(&mut self, changes: &[(WindowId, DockRect)]) {
        for (id, r) in changes {
            if let Some(slot) = self.rects.iter_mut().find(|(w, _)| w == id) {
                slot.1 = *r;
            }
        }
    }

    fn apply(&mut self, op: &Op) {
        let layout = self.layout();
        match *op {
            Op::Drag(i, dx, dy) if self.visible[i] => {
                let s = DragSession::start(IDS[i], (0.0, 0.0), &layout).unwrap();
                let moved = s.positions((dx, dy), &layout, &[], 10.0);
                self.set(&moved);
            }
            Op::Resize(i, w, h) if self.visible[i] && self.shaded[i].is_none() => {
                let min = min_inner_size(IDS[i]);
                let (w, h) = clamp_growth(IDS[i], w.max(min.x), h.max(min.y), &layout);
                self.set(&resize_with_attached(IDS[i], w, h, &layout));
            }
            Op::Shade(i) if self.visible[i] => {
                let r = self.rects[i].1;
                let h = match self.shaded[i].take() {
                    Some(h) => h,
                    None => {
                        self.shaded[i] = Some(r.height);
                        TITLE_HEIGHT
                    }
                };
                self.set(&resize_with_attached(IDS[i], r.width, h, &layout));
            }
            Op::Toggle(i) => {
                self.visible[i] = !self.visible[i];
                if self.visible[i] {
                    let others: Vec<DockRect> = layout.iter().map(|(_, r)| *r).collect();
                    self.rects[i].1 = separate_from(self.rects[i].1, &others);
                }
            }
            Op::DoubleSize => {
                let next = if self.zoom == 1.0 { 2.0 } else { 1.0 };
                let all = self.rects.clone();
                self.rects = rescale_layout(&all, WindowId::Main, self.zoom, next);
                self.zoom = next;
            }
            _ => {}
        }
    }

    fn assert_apart(&self, after: &Op) {
        let l = self.layout();
        for i in 0..l.len() {
            for j in i + 1..l.len() {
                assert!(
                    !overlaps(&l[i].1, &l[j].1),
                    "after {after:?}: {:?} overlaps {:?}",
                    l[i],
                    l[j]
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn no_two_visible_windows_ever_overlap(ops in proptest::collection::vec(op(), 1..40)) {
        let mut world = World::new();
        world.assert_apart(&Op::DoubleSize);
        for op in &ops {
            world.apply(op);
            world.assert_apart(op);
        }
    }
}
