#![allow(unused_imports)]
use libc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use x11::xlib::*;

type XColour = c_ulong;
const PATH: &str = "/home/blinklad/dev/rust/rdwm/src/config.toml";

// TODO Documentation for configuration options should follow this convention:
// https://github.com/rust-lang/rustfmt/blob/master/Configurations.md

/// Registers initial (root) window keybindings, colours and user preferences.
/// Holds runtime state of changes, if applicable.
/// Operations and data are mostly opaque to Rdwm proper, which is mainly just to _respond_ to events
/// by messaging appropriate handlers and handle any window-related book-keeping.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    windows: Option<ArrangementSettings>,
    borders: Option<BorderSettings>,
    #[serde(alias = "binding")]
    bindings: Option<Vec<KeySettings>>,
    #[serde(alias = "command")]
    commands: Option<Vec<CommandSettings>>,
    colour: Option<Vec<ColourSettings>>,
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
    colour: Option<String>,
    size: Option<usize>,
    focus_colour: Option<String>,
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
/// ```
/// [[binding]]
/// keys = [ "alt", "enter"]
/// operation = "term" # refers to 'term' key from [commands] table
///
/// [[binding]]
/// keys = [ "alt", "enter"]
/// operation = "kill focus" # refers to builtin command
/// ```
///
/// Keys are defined using a simplified version of the XBindKeys conventions.
#[derive(Debug, Serialize, Deserialize)]
struct KeySettings {
    name: Option<String>,
    keys: Option<Vec<String>>,
    operation: Option<String>,
}

/// [commands] section of configuration file.
/// Command settings are named values for to-be-executed commands, purely as a convenience for
/// keybinding and per-window rule settings.
/// [several](TODO) 'built-in' commands exist, such as ```kill focus```, ```kill all```, ```exec```.
/// User-supplied commands are (for the time being) assumed to run as narrowly POSIX compliant
/// shell scripts.
/// For example, in ```config.toml```:
/// ```
/// [[command]]
/// name = "term"
/// action = "exec alacritty"
///
/// [[command]]
/// name = "screenshot"
/// action = "scrot -s '%Y-%m-%d_$wx$h.png` -e"
/// ```
#[derive(Debug, Serialize, Deserialize)]
struct CommandSettings {
    name: Option<String>,
    action: Option<String>,
}

/// [colour] section of configuration file.
/// Colour settings are named values for user-defined colours or
/// [pre-defined XColours](https://en.wikipedia.org/wiki/X11_color_names#Color_name_chart).
/// This is purely a convenience for configuring rdwm settings, eg. border focus colour, pointer
/// colours and the like.
///
/// Rdwm expects colours as (maximum 64-bit) hexadecimal literals.
/// For example, in ```config.toml```:
/// ```
/// [[colour]]
/// name = "periwinkle blue"
/// value = 0xCCCCFF
///
/// [[colour]]
/// name = "burnt umber"
/// value = 0x8a3324
/// ```
#[derive(Debug, Serialize, Deserialize)]
struct ColourSettings {
    name: Option<String>,
    value: Option<XColour>,
}

impl Config {
    /// Produces a Rdwm configuration from either:
    /// 1. XDG base directory;
    /// 2. /etc/share/ defaults;
    /// 3. Application default values ('sensible' defaults)
    ///
    /// Once a base configuration is established, it may be the case that a well-formed
    /// config.toml file is invalid - eg. a colour or binding is named erroneously.
    /// In these cases, the configuration setting is ignored. However, this may cause cascading
    /// errors if another setting relies on this reference. An attempt to restore meaning will
    /// be made, however conservative it may be implemented as.
    ///
    /// Lastly, the result of a command (eg. exit status or IPC information) is not specified at
    /// this stage. It may be logged, but is likely ignored.
    ///
    pub fn get_config() -> Self {
        let config = PathBuf::from(PATH);
        let mut file = File::open(config).unwrap();
        let mut contents = String::new();
        let _bytes = file.read_to_string(&mut contents);
        let settings: Config = toml::from_str(&contents).unwrap();

        debug!("{:#?}", settings);
        settings
    }
}

#[test]
pub fn get_config() {
    let config = Config::get_config();
    println!("{:#?}", config);
}
