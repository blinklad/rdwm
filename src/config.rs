use libc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use x11::xlib::*;

type XColour = c_ulong; // TODO

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
    colours: Option<ColourSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
/// [arrangement] section of configuration file.
/// Arrangement settings are any settings that modify the size, behaviour or structure of client
/// windows. For example, size of inner gaps (default 0) or whether to ignore gaps for a single client
/// window on a workspace.
struct ArrangementSettings {
    inner_gap: Option<u8>,
    outer_gap: Option<u8>,
    smart_gaps: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
/// [border] section of configuration file.
/// Border settings are any settings that modify the size, appearance or behaviour of client window
/// borders. For example, the size of window borders (default 0) or colours for window urgency or
/// non-focussed windows.
struct BorderSettings {
    colour: Option<XColour>,
    size: Option<usize>,
    focus_colour: Option<XColour>,
    no_focus_colour: Option<bool>,
}

/// [binding] section of configuration file.
/// Binding settings are any settings that modify the behaviour of keystrokes globally.
/// Binding key _names_ are [pre-specified](TODO), and there are two built-in levels of precedence
/// for the _values_:
/// 1. Refers to an optional, user-supplied ```Operation``` by a named key; or
/// 2. Refers to a built-in operation (for example, close the focused window)
///
/// For example, in ```config.toml```:
///```
///[[binding]]
///name = "opens alacritty on alt+enter" # optional
///keys = [ "alt", "enter"]
///operation = "term" # refers to 'term' key from [commands] table
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

/// [commands] section of configuration file.
/// Command settings are named values for to-be-executed commands, purely as a convenience for
/// keybinding and per-window rule settings.
/// [several](TODO) 'built-in' commands exist, such as ```kill focus```, ```kill all```, ```exec```.
/// User-supplied commands are (for the time being) assumed to run as narrowly POSIX compliant
/// shell scripts.
/// For example, in ```config.toml```:
///```
///[commands]
///term = "exec alacritty"
///screenshot = "scrot -s '%Y-%m-%d_$wx$h.png` -e"
///```
#[derive(Debug, Serialize, Deserialize)]
struct CommandSettings {
    commands: Option<Vec<String>>,
}

/// [colour] section of configuration file.
/// Colour settings are named values for user-defined colours or
/// [pre-defined XColours](https://en.wikipedia.org/wiki/X11_color_names#Color_name_chart).
/// This is purely a convenience for configuring rdwm settings, eg. border focus colour, pointer
/// colours and the like.
///
/// Rdwm expects colours as (maximum 64-bit) hexadecimal literals.
/// For example, in ```config.toml```:
///```
///[colours]
///periwinkle_blue = 0xCCCCFF
///burnt_umber = 0x8a3324
///```
#[derive(Debug, Serialize, Deserialize)]
struct ColourSettings {
    colours: Option<Vec<XColour>>,
}

pub fn get_config() {}
