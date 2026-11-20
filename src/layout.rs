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

    /// Fingerprint the bounds, ordered typed entries and each rectangle in place.
    /// Reference capture bytes are host-test fixtures and do not enter the NRO.
    pub fn identify(
        screen: Screen,
        count: usize,
        bounds: [u64; 2],
        mut row: impl FnMut(usize) -> ([u64; 2], [u64; 2]),
    ) -> Option<Self> {
        let (layout, n, expected) = match screen {
            Screen::Css => (Self::Css, 87, 0x94d7_3f6e_a8ed_a7e2),
            Screen::Sss => (Self::Sss, 116, 0xd8c7_5452_465e_7857),
        };
        if count != n {
            return None;
        }
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut add = |word| {
            hash = (hash ^ word).wrapping_mul(0x100_0000_01b3);
        };
        for word in bounds {
            add(word);
        }
        for i in 0..n {
            let (kinds, rect) = row(i);
            add(kinds[0]);
            if screen == Screen::Css {
                add(kinds[1]);
            }
            add(rect[0]);
            add(rect[1]);
        }
        (hash == expected).then_some(layout)
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
