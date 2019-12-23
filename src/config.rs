#![allow(unused_imports, unused_variables, dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use x11::keysym::*;
use x11::xlib::*;

type ModifierMask = c_uint;
type XKeyFlags = c_uint;
type XColour = c_ulong;
type XKey = c_ulong;

const PATH: &str = "/home/blinklad/dev/rust/rdwm/src/config.toml";

/// Registers initial (root) window keybindings, colours and user preferences.
/// Holds runtime state of changes, if applicable.
/// Operations and data are mostly opaque to Rdwm proper, which is mainly just to _respond_ to events
/// by messaging appropriate handlers and handle any window-related book-keeping.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(alias = "windows")]
    window: Arrangement,
    #[serde(alias = "borders")]
    border: Border,
    #[serde(alias = "binding", flatten)]
    bindings: HashMap<KeyBinding, Action>,
    #[serde(alias = "command")]
    colour: Vec<Colour>,
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
        let settings: Config = toml::from_str(&contents).unwrap_or_default();

        debug!("{:#?}", settings);
        settings
    }
}

impl Default for Config {
    fn default() -> Self {
        info!("Using default configuration");

        let mut bindings = HashMap::new();
        bindings.insert(KeyBinding::default(), Action::default());

        Config {
            window: Default::default(),
            border: Default::default(),
            bindings,
            colour: vec![Colour::default()],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// [arrangement] section of configuration file.
/// Arrangement settings are any settings that modify the size, behaviour or structure of client
/// windows. For example, size of inner gaps (default 0) or whether to ignore gaps for a single client
/// window on a workspace.
struct Arrangement {
    inner_gap: u8,
    outer_gap: u8,
    smart_gaps: bool,
}

impl Default for Arrangement {
    fn default() -> Self {
        Arrangement {
            inner_gap: 0,
            outer_gap: 0,
            smart_gaps: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// [border] section of configuration file.
/// Border settings are any settings that modify the size, appearance or behaviour of client window
/// borders. For example, the size of window borders (default 0) or colours for window urgency or
/// non-focussed windows.
struct Border {
    colour: String,
    size: usize,
    focus_colour: String,
}

impl Default for Border {
    fn default() -> Self {
        Border {
            colour: String::from("black"),
            size: 1,
            focus_colour: String::from("white"),
        }
    }
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
#[derive(Debug, Serialize, Deserialize, Eq, Hash)]
struct KeyBinding {
    key: Key,
    mods: Vec<Modifier>,
}

impl PartialEq for KeyBinding {
    fn eq(&self, other: &Self) -> bool {
        self.get_keysym() == other.get_keysym()
    }
}

impl Default for KeyBinding {
    fn default() -> Self {
        KeyBinding {
            key: Key::NoKey,
            mods: Vec::new(),
        }
    }
}

// 97 keys
impl KeyBinding {
    fn get_mods(&self) -> &[Modifier] {
        self.mods.as_slice()
    }

    fn get_keysym(&self) -> KeySym {
        self.key.get_keysym()
    }
}

// Taken and modified from Alacritty
// Vim macros are OP
// https://github.com/jwilm/alacritty/blob/master/alacritty/src/config/bindings.rs
#[derive(Debug, Serialize, Deserialize, Eq, Hash)]
enum Key {
    #[serde(alias = "key1")]
    Key1,
    #[serde(alias = "key2")]
    Key2,
    #[serde(alias = "key3")]
    Key3,
    #[serde(alias = "key4")]
    Key4,
    #[serde(alias = "key5")]
    Key5,
    #[serde(alias = "key6")]
    Key6,
    #[serde(alias = "key7")]
    Key7,
    #[serde(alias = "key8")]
    Key8,
    #[serde(alias = "key9")]
    Key9,
    #[serde(alias = "key0")]
    Key0,
    #[serde(alias = "a")]
    A,
    #[serde(alias = "b")]
    B,
    #[serde(alias = "c")]
    C,
    #[serde(alias = "d")]
    D,
    #[serde(alias = "e")]
    E,
    #[serde(alias = "f")]
    F,
    #[serde(alias = "g")]
    G,
    #[serde(alias = "h")]
    H,
    #[serde(alias = "i")]
    I,
    #[serde(alias = "j")]
    J,
    #[serde(alias = "k")]
    K,
    #[serde(alias = "l")]
    L,
    #[serde(alias = "m")]
    M,
    #[serde(alias = "n")]
    N,
    #[serde(alias = "o")]
    O,
    #[serde(alias = "p")]
    P,
    #[serde(alias = "q")]
    Q,
    #[serde(alias = "r")]
    R,
    #[serde(alias = "s")]
    S,
    #[serde(alias = "t")]
    T,
    #[serde(alias = "u")]
    U,
    #[serde(alias = "v")]
    V,
    #[serde(alias = "w")]
    W,
    #[serde(alias = "x")]
    X,
    #[serde(alias = "y")]
    Y,
    #[serde(alias = "z")]
    Z,
    #[serde(alias = "escape")]
    Escape,
    #[serde(alias = "f1")]
    F1,
    #[serde(alias = "f2")]
    F2,
    #[serde(alias = "f3")]
    F3,
    #[serde(alias = "f4")]
    F4,
    #[serde(alias = "f5")]
    F5,
    #[serde(alias = "f6")]
    F6,
    #[serde(alias = "f7")]
    F7,
    #[serde(alias = "f8")]
    F8,
    #[serde(alias = "f9")]
    F9,
    #[serde(alias = "f10")]
    F10,
    #[serde(alias = "f11")]
    F11,
    #[serde(alias = "f12")]
    F12,
    #[serde(alias = "scroll")]
    Scroll,
    #[serde(alias = "pause")]
    Pause,
    #[serde(alias = "insert")]
    Insert,
    #[serde(alias = "home")]
    Home,
    #[serde(alias = "delete")]
    Delete,
    #[serde(alias = "end")]
    End,
    #[serde(alias = "page down")]
    PageDown,
    #[serde(alias = "page up")]
    PageUp,
    #[serde(alias = "left")]
    Left,
    #[serde(alias = "up")]
    Up,
    #[serde(alias = "right")]
    Right,
    #[serde(alias = "down")]
    Down,
    #[serde(alias = "back")]
    Back,
    #[serde(alias = "return")]
    Return,
    #[serde(alias = "space")]
    Space,
    #[serde(alias = "numlock")]
    Numlock,
    #[serde(alias = "numpad0")]
    Numpad0,
    #[serde(alias = "numpad1")]
    Numpad1,
    #[serde(alias = "numpad2")]
    Numpad2,
    #[serde(alias = "numpad3")]
    Numpad3,
    #[serde(alias = "numpad4")]
    Numpad4,
    #[serde(alias = "numpad5")]
    Numpad5,
    #[serde(alias = "numpad6")]
    Numpad6,
    #[serde(alias = "numpad7")]
    Numpad7,
    #[serde(alias = "numpad8")]
    Numpad8,
    #[serde(alias = "numpad9")]
    Numpad9,
    #[serde(alias = "apostrophe")]
    Apostrophe,
    #[serde(alias = "backslash")]
    Backslash,
    #[serde(alias = "colon")]
    Colon,
    #[serde(alias = "comma")]
    Comma,
    #[serde(alias = "grave")]
    Grave,
    #[serde(alias = "lAlt")]
    LAlt,
    #[serde(alias = "lBracket")]
    LBracket,
    #[serde(alias = "lControl")]
    LControl,
    #[serde(alias = "lShift")]
    LShift,
    #[serde(alias = "LWin")]
    LWin,
    #[serde(alias = "numpad comma")]
    NumpadComma,
    #[serde(alias = "numpad enter")]
    NumpadEnter,
    #[serde(alias = "numpad equals")]
    NumpadEquals,
    #[serde(alias = "period")]
    Period,
    #[serde(alias = "rAlt")]
    RAlt,
    #[serde(alias = "rBracket")]
    RBracket,
    #[serde(alias = "rControl")]
    RControl,
    #[serde(alias = "rShift")]
    RShift,
    #[serde(alias = "rWin")]
    RWin,
    #[serde(alias = "semicolon")]
    Semicolon,
    #[serde(alias = "slash")]
    Slash,
    #[serde(alias = "tab")]
    Tab,
    #[serde(skip)]
    NoKey,
}

impl Key {
    fn get_keysym(&self) -> KeySym {
        match self {
            Key::Key1 => XK_1.into(),
            Key::Key2 => XK_2.into(),
            Key::Key3 => XK_3.into(),
            Key::Key4 => XK_4.into(),
            Key::Key5 => XK_5.into(),
            Key::Key6 => XK_6.into(),
            Key::Key7 => XK_7.into(),
            Key::Key8 => XK_8.into(),
            Key::Key9 => XK_9.into(),
            Key::Key0 => XK_0.into(),
            Key::A => XK_A.into(),
            Key::B => XK_B.into(),
            Key::C => XK_C.into(),
            Key::D => XK_D.into(),
            Key::E => XK_E.into(),
            Key::F => XK_F.into(),
            Key::G => XK_G.into(),
            Key::H => XK_H.into(),
            Key::I => XK_I.into(),
            Key::J => XK_J.into(),
            Key::K => XK_K.into(),
            Key::L => XK_L.into(),
            Key::M => XK_M.into(),
            Key::N => XK_N.into(),
            Key::O => XK_O.into(),
            Key::P => XK_P.into(),
            Key::Q => XK_Q.into(),
            Key::R => XK_R.into(),
            Key::S => XK_S.into(),
            Key::T => XK_T.into(),
            Key::U => XK_U.into(),
            Key::V => XK_V.into(),
            Key::W => XK_W.into(),
            Key::X => XK_X.into(),
            Key::Y => XK_Y.into(),
            Key::Z => XK_Z.into(),
            Key::Escape => XK_Escape.into(),
            Key::F1 => XK_F1.into(),
            Key::F2 => XK_F2.into(),
            Key::F3 => XK_F3.into(),
            Key::F4 => XK_F4.into(),
            Key::F5 => XK_F5.into(),
            Key::F6 => XK_F6.into(),
            Key::F7 => XK_F7.into(),
            Key::F8 => XK_F8.into(),
            Key::F9 => XK_F9.into(),
            Key::F10 => XK_F10.into(),
            Key::F11 => XK_F11.into(),
            Key::F12 => XK_F12.into(),
            Key::Scroll => XK_Scroll_Lock.into(),
            Key::Pause => XK_Pause.into(),
            Key::Insert => XK_Pause.into(),
            Key::Home => XK_Home.into(),
            Key::Delete => XK_Delete.into(),
            Key::End => XK_End.into(),
            Key::PageDown => XK_Page_Down.into(),
            Key::PageUp => XK_Page_Up.into(),
            Key::Left => XK_Left.into(),
            Key::Up => XK_Up.into(),
            Key::Right => XK_Right.into(),
            Key::Down => XK_Down.into(),
            Key::Back => XK_BackSpace.into(),
            Key::Return => XK_Return.into(),
            Key::Space => XK_space.into(),
            Key::Numlock => XK_Num_Lock.into(),
            Key::Numpad0 => XK_KP_0.into(),
            Key::Numpad1 => XK_KP_1.into(),
            Key::Numpad2 => XK_KP_2.into(),
            Key::Numpad3 => XK_KP_3.into(),
            Key::Numpad4 => XK_KP_4.into(),
            Key::Numpad5 => XK_KP_5.into(),
            Key::Numpad6 => XK_KP_6.into(),
            Key::Numpad7 => XK_KP_7.into(),
            Key::Numpad8 => XK_KP_8.into(),
            Key::Numpad9 => XK_KP_9.into(),
            Key::Apostrophe => XK_apostrophe.into(),
            Key::Backslash => XK_backslash.into(),
            Key::Colon => XK_colon.into(),
            Key::Comma => XK_comma.into(),
            Key::Grave => XK_grave.into(),
            Key::LAlt => XK_Alt_L.into(),
            Key::LBracket => XK_bracketleft.into(),
            Key::LControl => XK_Control_L.into(),
            Key::LShift => XK_Shift_L.into(),
            Key::LWin => XK_Win_L.into(),
            Key::NumpadComma => XK_KP_Separator.into(), // https://www.cl.cam.ac.uk/~mgk25/ucs/keysymdef.h
            Key::NumpadEnter => XK_KP_Enter.into(),
            Key::NumpadEquals => XK_KP_Equal.into(),
            Key::Period => XK_period.into(),
            Key::RAlt => XK_Alt_R.into(),
            Key::RBracket => XK_bracketright.into(),
            Key::RControl => XK_Control_R.into(),
            Key::RShift => XK_Shift_R.into(),
            Key::RWin => XK_Win_R.into(),
            Key::Semicolon => XK_semicolon.into(),
            Key::Slash => XK_slash.into(),
            Key::Tab => XK_Tab.into(),
            Key::NoKey => panic!("No such key"),
        }
    }

    fn get_key_c_str(&self, key: XKey) -> &'static [u8] {
        // https://www.x.org/releases/X11R7.5/doc/man/man3/XStringToKeysym.3.html
        match self {
            Key::Key1 => b"1\0",
            Key::Key2 => b"2\0",
            Key::Key3 => b"3\0",
            Key::Key4 => b"4\0",
            Key::Key5 => b"5\0",
            Key::Key6 => b"6\0",
            Key::Key7 => b"7\0",
            Key::Key8 => b"8\0",
            Key::Key9 => b"9\0",
            Key::Key0 => b"0\0",
            Key::A => b"A\0",
            Key::B => b"B\0",
            Key::C => b"C\0",
            Key::D => b"D\0",
            Key::E => b"E\0",
            Key::F => b"F\0",
            Key::G => b"G\0",
            Key::H => b"H\0",
            Key::I => b"I\0",
            Key::J => b"J\0",
            Key::K => b"K\0",
            Key::L => b"L\0",
            Key::M => b"M\0",
            Key::N => b"N\0",
            Key::O => b"O\0",
            Key::P => b"P\0",
            Key::Q => b"Q\0",
            Key::R => b"R\0",
            Key::S => b"S\0",
            Key::T => b"T\0",
            Key::U => b"U\0",
            Key::V => b"V\0",
            Key::W => b"W\0",
            Key::X => b"X\0",
            Key::Y => b"Y\0",
            Key::Z => b"Z\0",
            Key::Escape => b"Escape\0",
            Key::F1 => b"F1\0",
            Key::F2 => b"F2\0",
            Key::F3 => b"F3\0",
            Key::F4 => b"F4\0",
            Key::F5 => b"F5\0",
            Key::F6 => b"F6\0",
            Key::F7 => b"F7\0",
            Key::F8 => b"F8\0",
            Key::F9 => b"F9\0",
            Key::F10 => b"F10\0",
            Key::F11 => b"F11\0",
            Key::F12 => b"F12\0",
            Key::Scroll => b"Scroll_Lock\0",
            Key::Pause => b"Pause\0",
            Key::Insert => b"Pause\0",
            Key::Home => b"Home\0",
            Key::Delete => b"Delete\0",
            Key::End => b"End\0",
            Key::PageDown => b"Page_Down\0",
            Key::PageUp => b"Page_Up\0",
            Key::Left => b"Left\0",
            Key::Up => b"Up\0",
            Key::Right => b"Right\0",
            Key::Down => b"Down\0",
            Key::Back => b"BackSpace\0",
            Key::Return => b"Return\0",
            Key::Space => b"space\0",
            Key::Numlock => b"Num_Lock\0",
            Key::Numpad0 => b"KP_0\0",
            Key::Numpad1 => b"KP_1\0",
            Key::Numpad2 => b"KP_2\0",
            Key::Numpad3 => b"KP_3\0",
            Key::Numpad4 => b"KP_4\0",
            Key::Numpad5 => b"KP_5\0",
            Key::Numpad6 => b"KP_6\0",
            Key::Numpad7 => b"KP_7\0",
            Key::Numpad8 => b"KP_8\0",
            Key::Numpad9 => b"KP_9\0",
            Key::Apostrophe => b"apostrophe\0",
            Key::Backslash => b"backslash\0",
            Key::Colon => b"colon\0",
            Key::Comma => b"comma\0",
            Key::Grave => b"grave\0",
            Key::LAlt => b"Alt_L\0",
            Key::LBracket => b"bracketleft\0",
            Key::LControl => b"Control_L\0",
            Key::LShift => b"Shift_L\0",
            Key::LWin => b"Win_L\0",
            Key::NumpadComma => b"KP_Separator\0",
            Key::NumpadEnter => b"KP_Enter\0",
            Key::NumpadEquals => b"KP_Equal\0",
            Key::Period => b"period\0",
            Key::RAlt => b"Alt_R\0",
            Key::RBracket => b"bracketright\0",
            Key::RControl => b"Control_R\0",
            Key::RShift => b"Shift_R\0",
            Key::RWin => b"Win_R\0",
            Key::Semicolon => b"semicolon\0",
            Key::Slash => b"slash\0",
            Key::Tab => b"Tab\0",
            Key::NoKey => panic!("No such key"),
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.get_keysym() == other.get_keysym()
    }
}

impl Default for Key {
    fn default() -> Self {
        Key::NoKey
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, Hash)]
enum Modifier {
    #[serde(alias = "super_l")]
    Super_L,
    #[serde(alias = "super_r")]
    Super_R,
    #[serde(alias = "alt_l")]
    Alt_L,
    #[serde(alias = "alt_r")]
    Alt_R,
    #[serde(alias = "shift_l")]
    Shift_L,
    #[serde(alias = "shift_r")]
    Shift_R,
    #[serde(alias = "caps_lock")]
    Caps_Lock,
    #[serde(alias = "shift")]
    Shift_Lock,
    #[serde(alias = "control_l")]
    Control_L,
    #[serde(alias = "control_r")]
    Control_R,
}

impl Modifier {
    fn get_keysym(&self) -> XKey {
        match self {
            Self::Super_L => XK_Super_L.into(),
            Self::Super_R => XK_Super_R.into(),
            Self::Alt_L => XK_Alt_L.into(),
            Self::Alt_R => XK_Alt_R.into(),
            Self::Shift_L => XK_Shift_L.into(),
            Self::Shift_R => XK_Shift_R.into(),
            Self::Caps_Lock => XK_Caps_Lock.into(),
            Self::Shift_Lock => XK_Shift_Lock.into(),
            Self::Control_L => XK_Control_L.into(),
            Self::Control_R => XK_Control_R.into(),
        }
    }

    fn get_key_c_str(&self) -> &'static [u8] {
        match self {
            Self::Super_L => b"Super_L\0",
            Self::Super_R => b"Super_R\0",
            Self::Alt_L => b"Alt_L\0",
            Self::Alt_R => b"Alt_R\0",
            Self::Shift_L => b"Shift_L\0",
            Self::Shift_R => b"Shift_R\0",
            Self::Caps_Lock => b"Caps_Lock\0",
            Self::Shift_Lock => b"Shift_Lock\0",
            Self::Control_L => b"Control_L\0",
            Self::Control_R => b"Control_R\0",
        }
    }
}

impl PartialEq for Modifier {
    fn eq(&self, other: &Self) -> bool {
        self.get_keysym() == other.get_keysym()
    }
}

impl Default for Modifier {
    fn default() -> Self {
        Modifier::Super_L
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Action {
    #[serde(alias = "full screen")]
    FullScreen,
    #[serde(alias = "minimize")]
    Minimize,
    #[serde(alias = "float focus")]
    FloatFocus,
    #[serde(alias = "ground focus")]
    GroundFocus,
    #[serde(alias = "kill focus")]
    KillFocus,
    #[serde(alias = "focus up")]
    MoveFocusUp,
    #[serde(alias = "focus down")]
    MoveFocusDown,
    #[serde(alias = "focus left")]
    MoveFocusLeft,
    #[serde(alias = "focus right")]
    MoveFocusRight,
    #[serde(alias = "split horizontal")]
    SplitHorizontal,
    #[serde(alias = "split vertical")]
    SplitVertical,
    #[serde(alias = "exit")]
    Exit,
    #[serde(alias = "move workspace")]
    MoveWorkspace(u32),
    #[serde(alias = "exec")]
    Execute(String),
    #[serde(skip)]
    NoAction,
}

impl Default for Action {
    fn default() -> Self {
        Action::NoAction
    }
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
struct Colour {
    name: String,
    value: XColour,
}

impl Default for Colour {
    fn default() -> Self {
        Colour {
            name: String::from("White"),
            value: 0xFFFFFF,
        }
    }
}

mod test {
    use crate::config::{Action, Config, KeyBinding, PATH};
    use serde_test::{assert_tokens, Token};
    use std::collections::HashMap;

    #[test]
    fn get_config() {
        let config = Config::get_config();
        println!("{:#?}", config);
    }

    #[test]
    fn test_keybinding() {
        let mut map = HashMap::new();
        map.insert(KeyBinding::default(), Action::default());
    }

    #[test]
    fn test_defaults() {}

    #[test]
    fn test_duplicate_binding() {}

    #[test]
    fn test_case() {}

    #[test]
    fn test_colour() {}

    #[test]
    fn test_negative_uint() {}

    #[test]
    fn test_command_lookup() {}
}
