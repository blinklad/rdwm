#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate bitflags;

use env_logger::WriteStyle::Auto;
use libc::*;
use std::sync::Mutex;
use x11::xlib::*;
type XWindow = x11::xlib::Window;

lazy_static! {
    /* TODO This could be a RefCell or any single-threaded sync primitive */
    static ref WM_DETECTED: Mutex<bool> = Mutex::new(false);
}

// Internal helper bitflags
bitflags! {
    struct WindowFlags: u32 {
        const NONE        = 1 << 0;
        const FIXED       = 1 << 1; // Hacky way to manipulate bars with Override Redirect set
        const FLOATING    = 1 << 2;
        const URGENT      = 1 << 3;
        const FULLSCREEN  = 1 << 4;
        const NEVER_FOCUS = 1 << 5;
    }
}

#[derive(Debug)]
struct Rdwm {
    display: *mut Display,
    root: XWindow,
    workspaces: Vec<Workspace>,
    current: usize,
}

impl Rdwm {
    fn init() -> Option<Self> {
        let display = unsafe {
            /* Safe because no side effects at this point */
            XOpenDisplay(std::ptr::null())
        };

        if display.is_null() {
            return None;
        }
        let screen = unsafe { XScreenOfDisplay(display, 0) };

        if screen as *mut _ == std::ptr::null_mut() {
            panic!("No screens associated with display");
        }

        /* TODO Screen indices _may_ work differently outside Xephyr */
        let root = unsafe { XDefaultRootWindow(display) };
        let mut workspaces = Vec::new();
        let cur_workspace = unsafe {
            Workspace::init(
                0,
                Quad::from_size((*screen).height as u32, (*screen).width as u32),
            )
        };

        debug!(
            "Display {:#?}\nRoot {:#?}\nSize:{:#?}",
            display, root, cur_workspace.screen
        );

        workspaces.push(cur_workspace);

        Some(Rdwm {
            display,
            root,
            workspaces,
            current: 0,
        })
    }

    fn get_current(&self) -> Option<&Workspace> {
        self.workspaces.get(self.current)
    }

    fn get_mut_current(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.current)
    }

    fn run(&mut self) {
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
        }

        unsafe {
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
            let existing = std::slice::from_raw_parts(existing_windows, num_existing as usize);

            for w in existing.iter() {
                self.frame(w, true);
            }
            XFree(existing_windows as *mut _ as *mut c_void);
            XUngrabServer(self.display);
        }

        loop {
            if *WM_DETECTED.lock().unwrap() == true {
                return;
            }

            let mut event: XEvent =
                unsafe { std::mem::MaybeUninit::<XEvent>::zeroed().assume_init() };
            unsafe {
                XNextEvent(self.display, &mut event);
            }

            #[allow(non_upper_case_globals)]
            unsafe {
                /* Safe because we know that the type of event dictates well-defined union member access */
                match event.get_type() { /* TODO */
                //  KeyPress =>
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
                _ => unimplemented!(),
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

    fn on_enter_notify(&mut self, event: &XCrossingEvent) {
        trace!("OnEnterNotify event: {:#?}", *event);

        /* Cloning for now even though its safe to borrow */
        let display_copy = self.display.clone();

        /* Very pythonic but should live elsewhere to prevent duplication */
        let (num, client) = self
            .get_current()
            .expect("No current")
            .clients
            .iter()
            .enumerate()
            .find(|(_, c)| c.frame.id == event.window)
            .expect("No such window");

        trace!("Client: {:#?} Number: {:#?}", client, num);

        self.get_mut_current()
            .expect("No current")
            .update_selected(display_copy, num);
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

        if let Some(client) = self
            .get_current()
            .expect("No workspaces")
            .clients
            .iter()
            .find(|c| (*c).context.id == (*event).window)
        {
            unsafe {
                XUnmapWindow(self.display, client.context.id);
                XReparentWindow(self.display, (*event).window, self.root, 0, 0);
                XRemoveFromSaveSet(self.display, (*event).window);
                XDestroyWindow(self.display, client.context.id);
                info!("Unframed client window: {:#?}", client);
            }
        } else {
            info!(
                "Ignoring UnmapNotify for non-client window: {:#?}",
                event.window
            );
            return;
        }

        self.get_mut_current()
            .unwrap()
            .clients
            .remove((*event).window as usize);
    }

    fn on_button_press(&self, event: &XButtonEvent) {
        trace!("OnButtonPress event: {:#?}", *event);
    }

    fn on_map_request(&mut self, event: &XMapRequestEvent) {
        self.frame(&(*event).window, false);
        info!("OnMapRequest event: {:#?}", *event);
    }

    fn frame(&mut self, window: &XWindow, already_existing: bool) {
        let window_attributes = unsafe {
            /* Safe as XGetWindowAttributes will write _something_ to result, and panic on bad request */
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
        let display_copy = self.display.clone();
        let root_copy = self.root.clone();

        self.get_mut_current().unwrap().create_window(
            display_copy,
            &root_copy,
            &window_attributes,
            &window,
        );

        unsafe {
            XAddToSaveSet(self.display, *window); /* offset */
        }

        self.get_current()
            .expect("No current")
            .arrange(self.display);

        trace!(
            "Created client: {:#?}",
            self.get_current().expect("No current").clients[0]
        );
    }

    fn on_configure_request(&self, event: &XConfigureRequestEvent) {
        info!("OnConfigureRequest event: {:#?}", *event);
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
        info!(
            "Resize window: {:#?} to {{ x: {} y: {} }}",
            event.window, event.width, event.height
        );
    }

    /* Static method, as Xlib needs to know the handlers address. */
    pub unsafe extern "C" fn on_wm_detected(
        _display: *mut Display,
        event: *mut XErrorEvent,
    ) -> c_int {
        assert_eq!(
            /* Currently panics with SIGILL, until more errors are handled */
            (*event).error_code,
            BadAccess,
            "Expected BadAccess error code OnWMDetected;
            Error handler not implemented for code: {:#?}",
            Rdwm::err_code_pretty((*event).error_code)
        );

        error!("Another window manager detected");

        let mut detected = WM_DETECTED.lock().unwrap();
        *detected = true;
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
    fn drop(&mut self) {
        unsafe {
            /* Safe because only 1 WM per x server */
            XCloseDisplay(self.display);
            info!("Closed display OK");
        }
    }
}

#[derive(Debug)]
struct Workspace {
    /* TODO Workspace buckets, hashable clients */
    number: usize,
    clients: Vec<Client>,
    selected: usize,
    screen: Quad,
}

impl Workspace {
    fn init(number: usize, screen: Quad) -> Self {
        Workspace {
            number,
            clients: Vec::new(),
            selected: 0,
            screen,
        }
    }

    #[allow(dead_code)]
    fn get_selected(&self) -> Option<&Client> {
        self.clients.get(self.selected)
    }

    #[allow(dead_code)]
    fn get_mut_selected(&mut self) -> Option<&mut Client> {
        self.clients.get_mut(self.selected)
    }

    fn update_selected(&mut self, display: *mut Display, index: usize) {
        let yellow = 0xEEE8AA;
        let blue = 0x5f316d;

        unsafe {
            trace!(
                "Change old border: {:#?}",
                XSetWindowBorder(display, self.clients[self.selected].frame.id, blue)
            );

            self.selected = index;
            trace!(
                "Set border result: {:#?}",
                XSetWindowBorder(display, self.clients[self.selected].frame.id, yellow)
            );
        }
    }

    #[allow(dead_code)]
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

    /* TODO Move simple tiling logic here */
    fn arrange(&self, display: *mut Display) {
        for (num, client) in self.clients.iter().enumerate() {
            trace!("{{ Num: {:#?} Client: {:#?} }}", num, *client);
            unsafe {
                trace!(
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
struct Client {
    name: String,
    frame: Window,
    context: Window,
    flags: WindowFlags,
}

impl Client {
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
struct Window {
    id: XWindow,
    hints: Attributes,
    attrs: Attributes,
}

impl Window {
    fn new(id: XWindow, attrs: &Quad, hints: &XWindowAttributes) -> Self {
        Window {
            id,
            hints: Attributes::new(&hints),
            attrs: Attributes::tiling(attrs),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
        Attributes {
            window: attrs.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Quad {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Quad {
    #[allow(dead_code)]
    fn default() -> Self {
        Quad {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }
    }

    #[allow(dead_code)]
    fn from_size(h: u32, w: u32) -> Self {
        Quad { x: 0, y: 0, w, h }
    }

    #[allow(dead_code)]
    fn from_coords(x: u32, y: u32) -> Self {
        Quad { x, y, h: 0, w: 0 }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .write_style(Auto)
        .init();
    info!("Starting logger OK");
    let mut rdwm = Rdwm::init()
        .ok_or("could not connect to display server")
        .unwrap();
    info!("Starting display server OK");
    rdwm.run();

    info!("Finish OK");
    Ok(())
}
