// TODO It could be pretty cute to use the include! macro + generated enumeration literals to specify
// Application logic a la dwm. This could interface with the system in a loosely coupled way to allow
// for a more robust approach in user-supplied configuration, eg. TOML, YAML, etc.
use libc::*;
use x11::xlib::*;

/// Registers initial (root) window keybindings, colours and user preferences.
/// Holds runtime state of changes, if applicable.
/// Operations and data are mostly opaque to Rdwm proper, which is mainly just to _respond_ to events
/// by messaging appropriate handlers and handle any window-related book-keeping.
type Colour = XColor;

struct RdwmConfig {}

struct WindowSettings {}

struct ArrangementSettings {
    InnerGapSize: usize,
    OuterGapSize: usize,
    SmartGapsOn: bool,
}

struct BorderSettings {
    BorderColour: Colour,
    BorderSize: usize,
    FocusColour: Colour,
    NoFocusColour: bool,
}
