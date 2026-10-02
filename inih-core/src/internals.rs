//! White-box helpers for tests ported from C static functions.
#![doc(hidden)]

use crate::{find_chars_or_comment, is_space, IniConfig};

pub fn ini_isspace(c: u8) -> bool {
    is_space(c)
}

pub fn ini_find_chars_or_comment(cfg: &IniConfig, s: &[u8], chars: Option<&[u8]>) -> usize {
    find_chars_or_comment(cfg, s, chars)
}
