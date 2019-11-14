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
    static ref WM_DETECTED: Mutex<bool> = Mutex::new(false);
}

bitflags! {
    struct WindowFlags: u32 {
        const NONE        = 0b00000000;
        const FIXED       = 0b00000001;
        const FLOATING    = 0b00000010;
        const URGENT      = 0b00000100;
        const FULLSCREEN  = 0b00001000;
        const NEVER_FOCUS = 0b00010000;
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
    fn get_current(&self) -> Option<&Workspace> {
        self.workspaces.get(self.current)
    }

    fn get_mut_current(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.current)
    }
}

#[derive(Debug)]
struct Workspace {
    number: u32,
    clients: Vec<Client>,
    selected: Option<Client>,
}

impl Workspace {
    fn init() -> Self {
        Workspace {
            number: 0,
            clients: Vec::new(),
            selected: None,
        }
    }
}

#[derive(Debug)]
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
        attrs: &XWindowAttributes,
        flags: WindowFlags,
    ) -> Self {
        let client = Client {
            name,
            frame: Window::new(frame, attrs),
            context: Window::new(context, attrs),
            flags,
        };
        trace!("Created client: {:#?}", client);
        client
    }
}

#[derive(Debug, Clone)]
struct Window {
    id: XWindow,
    attrs: Attributes,
}

impl Window {
    fn new(id: XWindow, attrs: &XWindowAttributes) -> Self {
        Window {
            id,
            attrs: Attributes::new(&attrs),
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
                x: attrs.x,
                y: attrs.y,
                h: attrs.height,
                w: attrs.width,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Quad {
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
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
        let root = unsafe { XDefaultRootWindow(display) };
        let mut workspaces = Vec::new();
        let cur_workspace = Workspace::init();
        workspaces.push(cur_workspace);

        debug!("Display {:?} Root {:?}", display, root);
        Some(Rdwm {
            display,
            root,
            workspaces,
            current: 0,
        })
    }

    fn run(&mut self) {
        unsafe {
            /* Safe as panics on errors that aren't handled properly yet */
            XSetErrorHandler(Some(Rdwm::on_wm_detected));

            /* We want to register reparenting for root window - If erroneous, handler will notify & exit */
            XSelectInput(
                self.display,
                self.root,
                SubstructureRedirectMask | SubstructureNotifyMask,
            );

            XSync(self.display, false as c_int);
        }

        unsafe {
            /* mem::zeroed is safe because XQueryTree will always write something, and panic on bad request */
            XGrabServer(self.display);
            let (mut existing_root, mut existing_parent): (XWindow, XWindow) =
                (std::mem::zeroed(), std::mem::zeroed());
            let (mut existing_windows, mut num_existing): (*mut XWindow, c_uint) =
                (std::mem::zeroed(), std::mem::zeroed());
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

            let mut event: XEvent = unsafe { std::mem::zeroed() };
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
                //  EnterNotify =>
                //  LeaveNotify =>
                //  FocusIn =>
                //  FocusOut =>
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
        let border_width: c_uint = 3;
        let border_color: c_ulong = 0x316d4c;
        let bg_color: c_ulong = 0x5f316d;

        let window_attributes = unsafe {
            /* Safe as XGetWindowAttributes will write _something_ to result, and panic on bad request */
            let mut attrs: XWindowAttributes = std::mem::zeroed();
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
                "Window already exists or map state is not viewable: {:#?}",
                window
            );
            return;
        }

        let frame = unsafe {
            XCreateSimpleWindow(
                self.display,
                self.root,
                0,
                0,
                window_attributes.width as c_uint,
                window_attributes.height as c_uint,
                border_width,
                border_color,
                bg_color,
            )
        };

        unsafe {
            XResizeWindow(self.display, *window, 800, 1080);
        }
        unsafe {
            XSelectInput(
                self.display,
                frame,
                SubstructureRedirectMask | SubstructureNotifyMask,
            );
            XAddToSaveSet(self.display, *window); /* offset */
            XReparentWindow(self.display, *window, frame, 0, 0);
            XMapWindow(self.display, frame);
            XMapWindow(self.display, *window);
            XGrabButton(
                self.display,
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
        }

        self.get_mut_current()
            .expect("No current")
            .clients
            .push(Client::new(
                String::from("0"),
                frame,
                *window,
                &window_attributes,
                WindowFlags::NONE,
            ));

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

        println!(
            "Current: {:#?} Clients: {:#?}",
            self.get_current().unwrap(),
            self.get_current().unwrap().clients
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
        } else {
            trace!("Couldn't find client: {:#?}", (*event).window);
        }
    }

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
