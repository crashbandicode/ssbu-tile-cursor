//! Ordered stock Training-grid identities and geometry from the qualified 13.0.5 release.
use crate::model::Screen;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    Css,
    Sss,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Css => "css.9f1fb03bf7c98fb9",
            Self::Sss => "sss.9892b3a32e5c0164",
        }
    }

    /// Match all native entry kinds, including CSS alternates, every rect and the hand clamp.
    /// The ID printed on the wire names this exact comparison; the game thread never hashes.
    pub fn identify(
        screen: Screen,
        count: usize,
        bounds: &[u8],
        mut matches: impl FnMut(usize, &[u8], &[u8]) -> bool,
    ) -> Option<Self> {
        let (layout, reference, kinds): (Layout, &[u8], usize) = match screen {
            Screen::Css => (Self::Css, include_bytes!("layouts/css.bin"), 16),
            Screen::Sss => (Self::Sss, include_bytes!("layouts/sss.bin"), 8),
        };
        let row = kinds + 16;
        if bounds != &reference[..16] || count != (reference.len() - 16) / row {
            return None;
        }
        reference[16..]
            .chunks_exact(row)
            .enumerate()
            .all(|(i, tile)| matches(i, &tile[..kinds], &tile[kinds..]))
            .then_some(layout)
    }
}

pub fn supported_css_entry(
    layout: Option<Layout>,
    position: (f32, f32),
    at: Option<u16>,
    raw_hover: Option<i32>,
) -> bool {
    layout == Some(Layout::Css)
        && position == (-599.0, -236.0)
        && at.is_none()
        && raw_hover == Some(-1)
}
