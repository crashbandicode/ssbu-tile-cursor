//! ssbu-tile-cursor: on the Smash Ultimate 13.0.5 character and stage select
//! screens, L3 + a D-pad direction moves P1's hand exactly one tile, to the
//! centre of the neighbouring fighter or stage tile. The game's own hand
//! easing does the move; tile rectangles are the ones the game hit-tests the
//! hand against. Without L3 nothing changes.
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
