//! Qualified local stage-select preview for the one native hand and slot.
use crate::layout::Layout;
use crate::provider::Step;

/// Start from a qualified rest and the next local step emitted after its target write.
/// The owner checks that target and the native preview again after Lua resumes.
pub fn start_move(
    rest: (u64, u64, u16),
    generation: u64,
    step: Step,
) -> Option<(u64, u64, u16, u16)> {
    let (rest_generation, previous, from) = rest;
    let to = step.to?;
    (step.ack == previous + 1
        && step.last_written == step.ack
        && step.from == Some(from)
        && rest_generation == generation
        && to != from
        && step.dir.is_some())
    .then_some((generation, step.ack, from, to))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    RandomNormal,
    RandomBattlefield,
    RandomOmega,
    Training,
    Battlefield,
    SmallBattlefield,
    BigBattlefield,
    FinalDestination,
}

impl Stage {
    // Random controls identify a picker action and have no concrete destination.
    pub fn id(self) -> Option<u16> {
        Some(match self {
            Self::RandomNormal | Self::RandomBattlefield | Self::RandomOmega => return None,
            Self::Training => 308,
            Self::Battlefield => 0,
            Self::SmallBattlefield => 347,
            Self::BigBattlefield => 2,
            Self::FinalDestination => 3,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::RandomNormal => "Random Normal",
            Self::RandomBattlefield => "Random Battlefield",
            Self::RandomOmega => "Random Omega",
            Self::Training => "Training",
            Self::Battlefield => "Battlefield",
            Self::SmallBattlefield => "Small Battlefield",
            Self::BigBattlefield => "Big Battlefield",
            Self::FinalDestination => "Final Destination",
        }
    }
}

pub fn stock_stage(layout: Layout, panel: u16) -> Option<Stage> {
    match (layout, panel) {
        (Layout::OrdinarySss, 0) => Some(Stage::RandomNormal),
        (Layout::OrdinarySss, 1) => Some(Stage::RandomBattlefield),
        (Layout::OrdinarySss, 2) => Some(Stage::RandomOmega),
        (Layout::OrdinarySss, 3) => Some(Stage::Battlefield),
        (Layout::OrdinarySss, 4) => Some(Stage::SmallBattlefield),
        (Layout::OrdinarySss, 5) => Some(Stage::BigBattlefield),
        (Layout::OrdinarySss, 6) => Some(Stage::FinalDestination),
        (Layout::Sss, 0) => Some(Stage::Training),
        (Layout::Sss, 1) => Some(Stage::Battlefield),
        (Layout::Sss, 2) => Some(Stage::SmallBattlefield),
        (Layout::Sss, 3) => Some(Stage::BigBattlefield),
        (Layout::Sss, 4) => Some(Stage::FinalDestination),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub struct Preview {
    pub kind: u64,
    pub form: i32,
    pub panel: i32,
}

#[derive(Clone, Copy)]
pub struct StageFacts {
    pub layout: Layout,
    pub tab: i64,
    pub slot: i64,
    pub getter: i32,
    pub actor_hover: i32,
    pub grid_kind: u64,
    pub records: [Preview; 3],
}

/// Stock r3 Proto, 84 little-endian instructions, SHA-256
/// ca61537ac8a382cd56ae9a8bc60178e69081ad03b6bb3994f11ea041f264fd3b.
pub fn actor_code_matches(code: &[u8]) -> bool {
    code.len() == 336
        && code
            .as_chunks::<4>().0.iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |hash, word| {
                let value = u32::from_le_bytes(*word);
                (hash ^ u64::from(value)).wrapping_mul(0x100_0000_01b3)
            })
            == 0x8b59_26a8_6bad_684e
}

/// Native preview forms from capture tc1, trace SHA-256 prefix 4ffa15f7.
pub fn preview_form(layout: Layout, panel: i32, kind: u64) -> i32 {
    match (layout, panel, kind) {
        (Layout::OrdinarySss, 1, 0x6900_021c_23e8_aab5) => 1,
        (Layout::OrdinarySss, 2, 0x6900_0313_bb95_239c) => 2,
        _ => 0,
    }
}

pub fn qualified_stage(s: StageFacts) -> Option<Stage> {
    let p = s.records[0];
    if s.tab != 0
        || s.slot != 0
        || s.getter != s.actor_hover
        || s.getter != p.panel
        || p.kind != s.grid_kind
        || p.form != preview_form(s.layout, p.panel, p.kind)
        || s.records[1..]
            .iter()
            .any(|other| other.kind != 0x69ff_ff00_0000_0000)
    {
        return None;
    }
    let panel = match s.layout {
        Layout::OrdinarySss => match (p.panel, p.kind) {
            (0, 0x6900_0116_6fbc_7641) => return Some(Stage::RandomNormal),
            (1, 0x6900_021c_23e8_aab5) => return Some(Stage::RandomBattlefield),
            (2, 0x6900_0313_bb95_239c) => return Some(Stage::RandomOmega),
            (3..=6, _) => p.panel - 2,
            _ => return None,
        },
        Layout::Sss => p.panel,
        Layout::Css => return None,
    };
    match (panel, p.kind) {
        (0, 0x6900_6d11_6b90_4091) => Some(Stage::Training),
        (1, 0x6900_0415_4b5c_4392) => Some(Stage::Battlefield),
        (2, 0x6900_8817_1954_c00c) => Some(Stage::SmallBattlefield),
        (3, 0x6900_0517_945c_cdf9) => Some(Stage::BigBattlefield),
        (4, 0x6900_060c_40bf_5050) => Some(Stage::FinalDestination),
        _ => None,
    }
}
