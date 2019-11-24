use libc::*;
use serde::{Deserialize, Serialize};
use serde_yaml::{Result, Value};
use x11::xlib::*;

// Glorious pseudocode from here onwards
// It could be pretty cute to use the include! macro + generated enumeration literals to specify
// Application logic a la dwm. This could interface with the system in a loosely coupled way to allow
// for a more robust approach in user-supplied configuration, eg. TOML, YAML, etc.

type Colour = XColor;
type Key = i32;

/// Registers initial (root) window keybindings, colours and user preferences.
/// Holds runtime state of changes, if applicable.
/// Operations and data are mostly opaque to Rdwm proper, which is mainly just to _respond_ to events
/// by messaging appropriate handlers and handle any window-related book-keeping.
struct RdwmConfig {}

struct WindowSettings {}

#[derive(Debug, Serialize, Deserialize)]
struct ArrangementSettings {
    inner_gap: u8,
    outer_gap: u8,
    smart_gaps: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct BorderSettings {
    border_colour: u32,
    border_size: usize,
    focus_colour: u32,
    no_focus_colour: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeySettings {}

pub fn get_config() {}
