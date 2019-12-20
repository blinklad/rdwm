#![allow(dead_code)]
use super::config::Config;
use libc::*;
use std::sync::Mutex;
use x11::keysym::*;

use x11::xlib::*;
type XWindow = x11::xlib::Window; // TODO NewType pattern to prevent i32 aliasing issues

lazy_static! {
    /// Lazily evaluated Mutex used to guard global error state required by Xlib error handler registration.
    /// It's not an ideal way to handle global state (even if it was changed to a more performant RefCell
    /// but will do for the time being.
    static ref WM_DETECTED: Mutex<bool> = Mutex::new(false);
}

bitflags! {
    /// 'Internal' bitflags (ie. not known to X) used to manage opt-in and default Client application logic.
    /// For example, the current state of a window to colour borders correctly, override tiling rules, etc.
    struct WindowFlags: u32 {
        const NONE         = 0 << 0;
        const TILING       = 1 << 0;
        const FLOATING     = 1 << 1;
        const URGENT       = 1 << 2;
        const FULLSCREEN   = 1 << 3;
        const NEVER_FOCUS  = 1 << 4;
        const FIXED        = 1 << 5; // Hacky way to manipulate bars with override redirect set
    }
}

/// Window manager that intercepts XEvents in the main event loop, propagating them to appropriate agents.
/// Maintains an XWindow handle registered for Substructure Redirection, as well as a collection of Workspaces
/// which hold client windows.
#[derive(Debug)]
pub struct Rdwm {
    display: *mut Display,
    root: XWindow,
    workspaces: Vec<Workspace>,
    current: usize,
    config: Config,
}

impl Rdwm {
    /// Instantiates a substructure redirected X client, with a single empty workspace.
    /// Refutable as there may already be an X client registered for substructure redirection (ie.
    /// another window manager).
    pub fn init() -> Option<Self> {
        let display = unsafe {
            /* Safe because no side effects at this point */
            XOpenDisplay(std::ptr::null())
        };

        if display.is_null() {
            return None;
        }
        let screen = unsafe { XScreenOfDisplay(display, 0) };

        if screen.is_null() {
            error!("No screens associated with display");
            return None;
        }

        // Grab config and register any changes to root window
        let config = Config::get_config();
        let root = Rdwm::register_root(&config, display);

        let mut workspaces = Vec::new();
        let cur_workspace = unsafe {
            Workspace::init(
                0,
                Quad::from_size((*screen).height as u32, (*screen).width as u32),
            )
        };

        workspaces.push(cur_workspace);

        Some(Rdwm {
            display,
            root,
            workspaces,
            current: 0,
            config,
        })
    }

    /// Returns a handle to an X display acting as the root window, registered for any configuration
    /// required by Rdwm consumers.
    fn register_root(_config: &Config, display: *mut Display) -> XWindow {
        // config.keys
        //       .for_each(|binding|
        //        XGrabKey(display, XKeysymToKeycode(display, binding.get_keysym()) as i32,
        //            binding.get_mods()
        //            root,
        //            false as c_int,
        //            GrabModeSync,
        //            GrabModeSync,
        //        );
        unsafe {
            let root = XDefaultRootWindow(display);
            XGrabKey(
                display,
                XKeysymToKeycode(display, XK_Return.into()) as i32,
                ControlMask | Mod1Mask,
                root,
                false as c_int,
                GrabModeSync,
                GrabModeSync,
            );
            XSelectInput(display, root, KeyPressMask); // TODO
            root
        }
    }

    /// Returns a shared reference to the current workspace. In situations of contention, eg. multiple
    /// monitors, the current workspace is a workspace such that the currently focused client window
    /// exists in said workspace.
    fn get_current(&self) -> Option<&Workspace> {
        self.workspaces.get(self.current)
    }

    /// Returns an exclusive reference to the current workspace. In situations of contention, eg. multiple
    /// monitors, the current workspace is a workspace such that the currently focused client window
    /// exists in said workspace.
    fn get_mut_current(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.current)
    }

    /// Begins the main event loop.
    /// Registers for error handling, input selection and synchronizes with the X server.
    pub fn run(&mut self) {
        unsafe {
            /* Sound, as panics on errors that aren't handled properly yet */
            XSetErrorHandler(Some(Rdwm::on_wm_detected));

            /* We want to register reparenting for root window - If erroneous, handler will notify & exit */
            XSelectInput(
                self.display,
                self.root,
                SubstructureRedirectMask | SubstructureNotifyMask | FocusChangeMask,
            );

            XSync(self.display, false as c_int);

            /* MaybeUninit is safe because XQueryTree will always write _something_ */
            XGrabServer(self.display);
            let mut existing_root = std::mem::MaybeUninit::<XWindow>::zeroed().assume_init();
            let mut existing_parent = std::mem::MaybeUninit::<XWindow>::zeroed().assume_init();
            let mut existing_windows =
                std::mem::MaybeUninit::<*mut XWindow>::zeroed().assume_init();
            let mut num_existing = std::mem::MaybeUninit::<c_uint>::zeroed().assume_init();

            assert!(
                XQueryTree(
                    self.display,
                    self.root,
                    &mut existing_root,
                    &mut existing_parent,
                    &mut existing_windows,
                    &mut num_existing
                ) != false as c_int,
                "Could not obtain existing query tree"
            );

            trace!(
                "Root: {:#?} Parent: {:#?} Windows: {:#?} Number of existing: {:#?}",
                existing_root,
                existing_parent,
                existing_windows,
                num_existing
            );

            assert_eq!(existing_root, self.root);

            // Frame existing windows from the saved set
            let existing = std::slice::from_raw_parts(existing_windows, num_existing as usize);
            for w in existing.iter() {
                self.frame(w, true);
            }

            XFree(existing_windows as *mut _ as *mut c_void);
            XUngrabServer(self.display);

            loop {
                if *WM_DETECTED.lock().unwrap() == true {
                    return;
                }

                let mut event: XEvent = { std::mem::MaybeUninit::<XEvent>::zeroed().assume_init() };

                XNextEvent(self.display, &mut event);

                #[allow(non_upper_case_globals)]
                /* Safe because we know that the type of event dictates well-defined union member access */
                match event.get_type() {
                    /* TODO */
                    KeyPress => self.on_key_press(&event.key),
                    //  KeyRelease =>
                    ButtonPress => self.on_button_press(&event.button),
                    //  ButtonRelease =>
                    //  MotionNotify =>
                    EnterNotify => self.on_enter_notify(&event.crossing),
                    LeaveNotify => self.on_leave(&event.crossing),
                    FocusIn => self.on_focus_in(&event.focus_change),
                    FocusOut => self.on_focus_in(&event.focus_change),
                    //  KeymapNotify =>
                    //  Expose =>
                    //  GraphicsExpose =>
                    //  NoExpose =>
                    //  VisibilityNotify =>
                    CreateNotify => self.on_create_notify(&event),
                    DestroyNotify => self.on_destroy_notify(&event.destroy_window),
                    UnmapNotify => self.on_unmap_notify(&event.unmap),
                    MapNotify => self.on_map_notify(&event.map),
                    MapRequest => self.on_map_request(&event.map_request),
                    ReparentNotify => self.on_reparent_notify(&event.reparent),
                    ConfigureNotify => self.on_configure_notify(&event.configure),
                    ConfigureRequest => self.on_configure_request(&event.configure_request),
                    //  GravityNotify =>
                    //  ResizeRequest =>
                    //  CirculateNotify =>
                    //  CirculateRequest =>
                    //  PropertyNotify =>
                    //  SelectionClear =>
                    //  SelectionRequest =>
                    //  SelectionNotify =>
                    //  ColormapNotify =>
                    //  ClientMessage =>
                    //  MappingNotify =>
                    //  GenericEvent =>
                    _ => unimplemented!("{:#?}", event),
                }
            }
        }
    }

    fn on_create_notify(&self, event: &XEvent) {
        trace!("OnCreateNotify event: {:#?}", *event);
    }

    fn on_destroy_notify(&self, event: &XDestroyWindowEvent) {
        trace!("XDestroyWindowEvent event: {:#?}", *event);
    }

    fn on_reparent_notify(&self, event: &XReparentEvent) {
        trace!("OnReparentNotify event: {:#?}", *event);
    }

    fn on_map_notify(&self, event: &XMapEvent) {
        trace!("OnMapNotify event: {:#?}", *event);
    }

    fn on_configure_notify(&self, event: &XConfigureEvent) {
        trace!("OnConfigureNotify event: {:#?}", *event);
    }

    fn on_key_press(&self, event: &XKeyEvent) {
        unsafe {
            if (*event).keycode == XKeysymToKeycode(self.display, XK_Return.into()).into() {
                XUngrabKey(
                    self.display,
                    XKeysymToKeycode(self.display, XK_Return.into()) as i32,
                    ControlMask | Mod1Mask,
                    self.root,
                );
            }
        }
        trace!("OnKeyPress event: {:#?}", *event);
    }

    fn on_enter_notify(&mut self, event: &XCrossingEvent) {
        trace!("OnEnterNotify event: {:#?}", *event);

        /* Cloning for now even though its safe to borrow */
        let display_copy = self.display;

        /* Very pythonic but should live elsewhere to prevent duplication */
        if let Some((num, client)) = self
            .get_current()
            .expect("No current")
            .clients
            .iter()
            .enumerate()
            .find(|(_, c)| c.frame.id == event.window)
        {
            trace!("Client: {:#?} Number: {:#?}", client, num);

            self.get_mut_current()
                .expect("No current")
                .update_selected(display_copy, num);
        } else {
            return;
        }
    }

    fn on_leave(&self, event: &XCrossingEvent) {
        trace!("OnLeaveNotify event: {:#?}", *event);
    }

    fn on_focus_in(&mut self, event: &XFocusChangeEvent) {
        trace!("OnFocusIn event: {:#?}", *event);
    }

    fn on_unmap_notify(&mut self, event: &XUnmapEvent) {
        trace!("OnUnmapNotify event: {:#?}", *event);

        if (*event).event == self.root {
            info!("Ignoring UnmapNotify for existing window");
            return;
        }

        let (num, _) = self
            .get_current()
            .expect("No workspaces")
            .clients
            .iter()
            .enumerate()
            .find(|(_, c)| (*c).context.id == (*event).window)
            .expect("No such item");
        {
            let display = self.display;
            let root = self.root;

            self.get_mut_current()
                .expect("No such workspace")
                .destroy_window(display, root, num);
        }
    }

    fn on_button_press(&self, event: &XButtonEvent) {
        trace!("OnButtonPress event: {:#?}", *event);
    }

    fn on_map_request(&mut self, event: &XMapRequestEvent) {
        self.frame(&(*event).window, false);
        trace!("OnMapRequest event: {:#?}", *event);
    }

    /// Given a client window, create and reparent the client within a top-level frame, setting
    /// appropriate client window hints in the process.
    fn frame(&mut self, window: &XWindow, already_existing: bool) {
        /* Safe as XGetWindowAttributes will write _something_ to result, and panic on bad request */
        let window_attributes = unsafe {
            let mut attrs = std::mem::MaybeUninit::<XWindowAttributes>::zeroed().assume_init();
            let ok = XGetWindowAttributes(self.display, *window, &mut attrs);

            trace!("Window attributes: {:#?}", ok);
            assert!(ok != 0, "Could not acquire window attributes");
            attrs
        };

        if already_existing
            && (window_attributes.override_redirect != 0
                || window_attributes.map_state != IsViewable)
        {
            trace!(
                "Window already exists, map state is not viewable, or override redirect set: {:#?}",
                window
            );
            return;
        };

        /* Cloning for now even though its safe to borrow */
        let display_copy = self.display;
        let root_copy = self.root;

        self.get_mut_current().unwrap().create_window(
            display_copy,
            &root_copy,
            &window_attributes,
            &window,
        );

        unsafe {
            XAddToSaveSet(self.display, *window);
        }

        self.get_current()
            .expect("No current")
            .arrange(self.display);
    }

    /// Configure a client window based on given hints.
    fn on_configure_request(&self, event: &XConfigureRequestEvent) {
        trace!("OnConfigureRequest event: {:#?}", *event);

        let mut config = XWindowChanges {
            x: event.x,
            y: event.y,
            width: event.width,
            height: event.height,
            border_width: event.border_width,
            sibling: event.above,
            stack_mode: event.detail,
        };
        debug!(
            "XWindowChanges: {:#?} for Window: {:#?}",
            config,
            (*event).window
        );

        if let Some(client) = self
            .get_current()
            .expect("No current")
            .clients
            .iter()
            .find(|c| c.context.id == (*event).window)
        {
            /* re-configure existing frame */
            unsafe {
                XConfigureWindow(
                    self.display,
                    client.frame.id,
                    event.value_mask as u32,
                    &mut config,
                );
            };
        }
        /* configure client window */
        unsafe {
            XConfigureWindow(
                self.display,
                event.window,
                event.value_mask as u32,
                &mut config,
            );
        };
        trace!(
            "Resized window: {:#?} to {{ x: {} y: {} }}",
            event.window,
            event.width,
            event.height
        );
    }

    /// Static method to interface with X's error handling routines.
    /// Currently only handles BadAccess errors raised when, on running Rdwm, another X client exists
    /// that has registered for substructure redirection (ie. another window manager).
    pub unsafe extern "C" fn on_wm_detected(
        _display: *mut Display,
        _event: *mut XErrorEvent,
    ) -> c_int {
        //assert_eq!(
        //    /* Currently panics with SIGILL, until more errors are handled */
        //    (*event).error_code,
        //    BadAccess,
        //    "Expected BadAccess error code OnWMDetected;
        //    Error handler not implemented for code: {:#?}",
        //    Rdwm::err_code_pretty((*event).error_code)
        //);

        error!("Another window manager detected");

        //let mut detected = WM_DETECTED.lock().unwrap();
        //*detected = true;
        0 /* This is ignored */
    }

    fn err_code_pretty(code: c_uchar) -> &'static str {
        match code {
            0 => "Success",
            1 => "BadRequest",
            2 => "BadValue",
            3 => "BadWindow",
            4 => "BadPixmap",
            5 => "BadAtom",
            6 => "BadCursor",
            7 => "BadFont",
            8 => "BadMatch",
            9 => "BadDrawable",
            10 => "BadAccess",
            11 => "BadAlloc",
            12 => "BadColor",
            13 => "BadGC",
            14 => "BadIDChoice",
            15 => "BadName",
            16 => "BadLength",
            17 => "BadImplementation",
            128 => "FirstExtensionError",
            255 => "LastExtensionError",
            _ => "Unknown error code",
        }
    }
}

impl Drop for Rdwm {
    /// Ensure that when event loop is exited through well-defined behaviour (eg. stack unwinding,
    /// normal exit or X server requests) that the display handle is closed.
    fn drop(&mut self) {
        unsafe {
            /* Safe because only 1 WM per x server */
            XCloseDisplay(self.display);
            info!("Closed display OK");
        }
    }
}

#[derive(Debug)]
/// Workspaces form the core abstraction over a group of client windows whose arrangements affect
/// that of their peers (unless they are floating or fixed).
/// Individual workspaces manage their own clients, including book-keeping the currently selected
/// client, floating and fixed clients and details specific to the (logical) screen that they reside
/// on. Not quite analogous to dwm's Monitor but holds some similarities.
struct Workspace {
    // TODO a MxN matrix with client indices better represents
    // the abstraction of window arrangement compared to a hierarchical approach
    number: usize,
    clients: Vec<Client>,
    selected: usize,
    floating: usize,
    screen: Quad,
}

impl Workspace {
    /// Create an empty workspace of a given size.
    fn init(number: usize, screen: Quad) -> Self {
        Workspace {
            number,
            clients: Vec::new(),
            selected: 0,
            floating: 0,
            screen,
        }
    }

    /// Returns a shared reference to the currently selected client.
    fn get_selected(&self) -> Option<&Client> {
        self.clients.get(self.selected)
    }

    /// Returns an exclusive reference to the currently selected client.
    fn get_mut_selected(&mut self) -> Option<&mut Client> {
        self.clients.get_mut(self.selected)
    }

    /// Update the workspaces currently selected client, including re-decorating window frames.
    fn update_selected(&mut self, display: *mut Display, index: usize) {
        // TODO Use the type system to enforce indices belonging to the Client collection.
        let yellow = 0xEEE8AA;
        let blue = 0x5f316d;

        unsafe {
            /* If the index is greater, then it's an unmapped window we don't care about*/
            self.selected = {
                if self.clients.len() > self.selected {
                    trace!(
                        "Change old border: {:#?}",
                        XSetWindowBorder(display, self.clients[self.selected].frame.id, blue)
                    );
                    index
                } else {
                    /* "Sensible" default of MRU window */
                    self.clients.len() - 1
                }
            };

            trace!(
                "Set border result: {:#?}",
                XSetWindowBorder(display, self.clients[self.selected].frame.id, yellow)
            );
        }
    }

    /// Creates a window for an X client.
    /// The window is registered for substructure redirection, focus change and enter / leave events,
    fn create_window(
        &mut self,
        display: *mut Display,
        root: &XWindow,
        attrs: &XWindowAttributes,
        window: &XWindow,
    ) {
        let border_width: c_uint = 3;
        let border_color: c_ulong = 0x316d4c;
        let bg_color: c_ulong = 0x5f316d;

        unsafe {
            let frame = XCreateSimpleWindow(
                display,
                *root,
                0, //(self.clients.len() * (self.screen.w as usize / 2 * self.clients.len())) as i32
                0,
                (self.screen.w / 2) as c_uint,
                (self.screen.h) as c_uint,
                border_width,
                border_color,
                bg_color,
            );

            XSelectInput(
                display,
                frame,
                SubstructureRedirectMask
                    | SubstructureNotifyMask
                    | FocusChangeMask
                    | EnterWindowMask
                    | LeaveWindowMask,
            );

            XReparentWindow(display, *window, frame, 0, 0);
            XMapWindow(display, frame);
            XMapWindow(display, *window);
            XGrabButton(
                display,
                Button1,
                ShiftMask,
                *window,
                0,
                0,
                GrabModeSync,
                GrabModeSync,
                *window,
                0x0,
            );

            self.clients.push(Client::new(
                String::from("0"),
                frame,
                *window,
                &attrs,
                &Quad::from_size(self.screen.h, self.screen.w),
                WindowFlags::NONE,
            ));
        }
    }

    /// Destroys an X client window. The window (and its frame) are unmapped and destroyed by X.
    /// Then, the workspace that the client belongs to is rearranged.
    fn destroy_window(&mut self, display: *mut Display, root: XWindow, index: usize) {
        let client = &mut self.clients[index];

        // TODO
        unsafe {
            XUnmapWindow(display, client.context.id);
            XUnmapWindow(display, client.frame.id);
            XReparentWindow(display, client.context.id, root, 0, 0);
            XDestroyWindow(display, client.context.id);
            XDestroyWindow(display, client.frame.id);
        };

        self.clients.remove(index);
        self.arrange(display); // TODO What if a Client is destroyed on a different workspace than
                               // the currently selected workspace?
    }

    /// Refresh client windows on a workspace to match some arrangement, eg. tiling over the screen
    /// space.
    fn arrange(&self, display: *mut Display) {
        trace!("Arranging client/s");
        for (num, client) in self.clients.iter().enumerate() {
            info!("{{ Num: {:#?} Client: {:#?} }}", num, *client);
            unsafe {
                info!(
                    "Offset: {:#?}",
                    ((num) * (*client).frame.attrs.window.w as usize / self.clients.len()) as i32
                );
                XMoveResizeWindow(
                    display,
                    client.frame.id,
                    ((num) * (*client).frame.attrs.window.w as usize / self.clients.len()) as i32,
                    0,
                    self.screen.w / (self.clients.len() as u32),
                    self.screen.h,
                );

                XMoveResizeWindow(
                    display,
                    client.context.id,
                    0,
                    0,
                    self.screen.w / (self.clients.len() as u32),
                    self.screen.h,
                );
                XMapWindow(display, client.frame.id);
                XMapWindow(display, client.context.id);
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Clients represent an XWindow frame + client pairing, with additional context and attributes for
/// book-keeping, eg. window size hints, fixed, floating etc.
struct Client {
    name: String,
    frame: Window,
    context: Window,
    flags: WindowFlags,
}

impl Client {
    // TODO Perhaps Builder pattern would work well here.
    /// Create a window that shall be tiled.
    fn tile(
        name: String,
        frame: XWindow,
        context: XWindow,
        hints: &XWindowAttributes,
        attrs: &Quad,
    ) -> Self {
        Client {
            name,
            frame: Window::new(frame, attrs, hints),
            context: Window::new(context, attrs, hints),
            flags: WindowFlags::TILING,
        }
    }
    /// Create a window that shall be floating.
    fn floating(
        name: String,
        frame: XWindow,
        context: XWindow,
        hints: &XWindowAttributes,
        attrs: &Quad,
    ) -> Self {
        Client {
            name,
            frame: Window::new(frame, attrs, hints),
            context: Window::new(context, attrs, hints),
            flags: WindowFlags::FLOATING,
        }
    }
    /// Create a window that shall have any flags passed in.
    fn new(
        name: String,
        frame: XWindow,
        context: XWindow,
        hints: &XWindowAttributes,
        attrs: &Quad,
        flags: WindowFlags,
    ) -> Self {
        Client {
            name,
            frame: Window::new(frame, attrs, hints),
            context: Window::new(context, attrs, hints),
            flags,
        }
    }
}

#[derive(Debug, Clone)]
/// Rdwm Windows are a thin wrapper around XWindows (which are really just a number used for X's
/// book-keeping). Rdwm windows contain client-supplied hints (eg. values for screen positioning
/// that may be used when toggling between floating / tiling modes) and _actual_ values supplied to
/// X when mapping and resizing Client windows based on a Workspace.
struct Window {
    id: XWindow,
    hints: Attributes,
    attrs: Attributes,
}

impl Window {
    /// Create a new Window.
    fn new(id: XWindow, attrs: &Quad, hints: &XWindowAttributes) -> Self {
        Window {
            id,
            hints: Attributes::new(&hints),
            attrs: Attributes::tiling(attrs),
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Attributes are a very thin wrapper around a 4-tuple of co-ordinates and sizes used to plot
/// Clients onto a logical screen. This struct will certainly be changed in the near future to
/// be more extensible to user- and developer-supplied values.
struct Attributes {
    window: Quad,
}

impl Attributes {
    fn new(attrs: &XWindowAttributes) -> Self {
        Attributes {
            window: Quad {
                x: attrs.x as u32,
                y: attrs.y as u32,
                h: attrs.height as u32,
                w: attrs.width as u32,
            },
        }
    }

    fn tiling(attrs: &Quad) -> Self {
        Attributes { window: *attrs }
    }
}

#[derive(Debug, Clone, Copy)]
/// A 4-tuple of integers used to plot a point on a screen as a co-ordinate vector.
struct Quad {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Quad {
    fn default() -> Self {
        Quad {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }
    }

    fn from_size(h: u32, w: u32) -> Self {
        Quad { x: 0, y: 0, w, h }
    }

    fn from_coords(x: u32, y: u32) -> Self {
        Quad { x, y, h: 0, w: 0 }
    }
}
