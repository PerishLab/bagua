use crate::{Extent, Failure};

pub const SOURCE: &str = "vt100:fb0f54ec7222f4805f33cd3ca530229e0a0fe6ad";
pub const COLUMNS: u16 = 512;
pub const ROWS: u16 = 256;
pub const HISTORY: usize = 50_000;
pub const CELLS: usize = 1_048_576;
pub const CHUNK: usize = 4096;

pub const PROFILE: &[&str] = &[
    "streaming UTF-8 with bounded combining cells and CJK continuation",
    "cursor movement, save/restore, erase, insert/delete and scrolling regions",
    "primary/alternate screens, autowrap and application cursor/keypad modes",
    "indexed/RGB colors, bold, dim, italic, underline and inverse",
    "bounded primary history and immutable revision-bound snapshots",
    "legacy arrows, navigation, F1-F12, control keys and bracketed paste",
    "CSI-u supported-mask flag 1 negotiation with separate bounded primary/alternate stacks",
    "typed Shift/Alt/Ctrl keys and unambiguous modified navigation/F1-F12",
    "X10/normal/drag/motion tracking with legacy or SGR mouse encoding",
    "primary device/status/cursor/character-size queries, explicit replies and bell counts",
];

pub const EXCLUDED: &[&str] = &[
    "DECAWM switching; wrapping stays enabled in this profile",
    "full grapheme shaping, emoji ZWJ and unbounded combining sequences",
    "history reflow when changing column count",
    "UTF-8 mouse encoding; keyboard enhancement flags 2/4/8/16, lock/super/media/keypad keys",
    "OSC metadata, clipboard, hyperlinks, images and DCS extensions",
    "application-requested window resize and external terminal side effects",
    "malformed UTF-8 replacement display and full VT standards conformance",
];

pub(crate) fn check(extent: Extent, history: usize) -> Result<(), Failure> {
    if extent.width() < 2 || extent.width() > COLUMNS || extent.height() > ROWS {
        return Err(Failure::Capacity);
    }
    if history > HISTORY {
        return Err(Failure::Capacity);
    }
    let rows = usize::from(extent.height()) * 2;
    if rows * usize::from(extent.width()) > CELLS {
        return Err(Failure::Capacity);
    }
    Ok(())
}
