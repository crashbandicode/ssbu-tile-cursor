//! Qualified local Training stage preview for the one native hand and slot.
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
    Training,
    Battlefield,
    SmallBattlefield,
    BigBattlefield,
    FinalDestination,
}

impl Stage {
    pub fn id(self) -> u16 {
        match self {
            Self::Training => 308,
            Self::Battlefield => 0,
            Self::SmallBattlefield => 347,
            Self::BigBattlefield => 2,
            Self::FinalDestination => 3,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Training => "Training",
            Self::Battlefield => "Battlefield",
            Self::SmallBattlefield => "Small Battlefield",
            Self::BigBattlefield => "Big Battlefield",
            Self::FinalDestination => "Final Destination",
        }
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

pub fn qualified_stage(s: StageFacts) -> Option<Stage> {
    let p = s.records[0];
    if s.tab != 0
        || s.slot != 0
        || s.getter != s.actor_hover
        || s.getter != p.panel
        || p.kind != s.grid_kind
        || p.form != 0
        || s.records[1..]
            .iter()
            .any(|other| other.kind != 0x69ff_ff00_0000_0000)
    {
        return None;
    }
    match (p.panel, p.kind) {
        (0, 0x6900_6d11_6b90_4091) => Some(Stage::Training),
        (1, 0x6900_0415_4b5c_4392) => Some(Stage::Battlefield),
        (2, 0x6900_8817_1954_c00c) => Some(Stage::SmallBattlefield),
        (3, 0x6900_0517_945c_cdf9) => Some(Stage::BigBattlefield),
        (4, 0x6900_060c_40bf_5050) => Some(Stage::FinalDestination),
        _ => None,
    }
}
