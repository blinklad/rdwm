use libc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use x11::xlib::*;

lazy_static! {
    /* TODO XDG base directory compliance */
    static ref CONFIG: PathBuf = PathBuf::from("~/dev/rust/rdwm/src/config.toml");
}

/// Registers initial (root) window keybindings, colours and user preferences.
/// Holds runtime state of changes, if applicable.
/// Operations and data are mostly opaque to Rdwm proper, which is mainly just to _respond_ to events
/// by messaging appropriate handlers and handle any window-related book-keeping.
#[derive(Debug, Serialize, Deserialize)]
struct RdwmConfig {
    windows: Option<ArrangementSettings>,
    borders: Option<BorderSettings>,
    bindings: Option<KeySettings>,
}

#[derive(Debug, Serialize, Deserialize)]
/// [Arrangement] section of configuration file.
/// Arrangement settings are any settings that modify the size, behaviour or structure of client
/// windows. For example, size of inner gaps (default 0) or whether to ignore gaps for a single client
/// window on a workspace.
struct ArrangementSettings {
    inner_gap: Option<u8>,
    outer_gap: Option<u8>,
    smart_gaps: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
/// [Border] section of configuration file.
/// Border settings are any settings that modify the size, appearance or behaviour of client window
/// borders. For example, the size of window borders (default 0) or colours for window urgency or
/// non-focussed windows.
struct BorderSettings {
    border_colour: Option<u32>,
    border_size: Option<usize>,
    focus_colour: Option<u32>,
    no_focus_colour: Option<bool>,
}

/// [Binding] section of configuration file.
/// Binding settings are any settings that modify the behaviour of keystrokes globally.
/// Binding key _names_ are [pre-specified](TODO), and there are two built-in levels of precedence
/// for the _values_:
/// 1. Refers to an optional, user-supplied operation (assumed to be a shell script);
/// 2. Otherwise, refers to a built-in behaviour (for example, close the focused window)
///
/// For example, in ```config.toml```:
///```
///[[binding]]
///name = "open window" # Optional
///keys = [ "alt", "enter"]
///operation = "exec alacritty"
///builtin = false
///
///[[binding]]
///keys = [ "alt", "enter"]
///operation = "kill focus"
///```
///
/// Keys are defined using a simplified version of the XBindKeys conventions.
#[derive(Debug, Serialize, Deserialize)]
struct KeySettings {
    binding: Option<Vec<HashMap<String, String>>>,
}

pub fn get_config() {}
