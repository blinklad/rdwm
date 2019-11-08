#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use env_logger::WriteStyle::Auto;
use libc::*;
use std::collections::HashMap;
use std::sync::Mutex;
use x11::xlib::*;

lazy_static! {
    static ref WM_DETECTED: Mutex<bool> = Mutex::new(false);
}

#[derive(Debug)]
struct Rdwm {
    display: *mut Display,
    root: Window,
    clients: HashMap<Window, Window>, /* Window -> Frame*/
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
        let clients = HashMap::new();

        debug!("Display {:?} Root {:?}", display, root);
        info!("Display {:?} Root {:?}", display, root);
        Some(Rdwm {
            display,
            root,
            clients,
        })
    }

    fn run(&mut self) {
        unsafe {
            /* Safe because TODO */
            XSetErrorHandler(Some(Rdwm::on_wm_detected));

            /* We want to register reparenting for root window - If erroneous, handler will notify */
            XSelectInput(
                self.display,
                self.root,
                SubstructureRedirectMask | SubstructureNotifyMask,
            );

            XSync(self.display, false as c_int);
        }

        // TODO This needs big fix
        //unsafe {
        //    /* This is certainly a gross amount of side effects that I hope XCB does better */
        //    XGrabServer(self.display);
        //    let (existing_root, existing_parent): (*mut Window, *mut Window) =
        //        (std::mem::zeroed(), std::mem::zeroed());
        //    let (existing_windows, num_existing): (*mut *mut Window, *mut c_uint) =
        //        (std::mem::zeroed(), std::mem::zeroed());

        //    assert!(
        //        XQueryTree(
        //            self.display,
        //            self.root,
        //            existing_root,
        //            existing_parent,
        //            existing_windows,
        //            num_existing
        //        ) != false as c_int,
        //        "Could not obtain existing query tree"
        //    );

        //    assert_eq!(*existing_root, self.root);
        //    let existing = std::slice::from_raw_parts(*existing_windows, *num_existing as usize);

        //    for w in existing.iter() {
        //        self.frame(w, true);
        //    }
        //    XFree(existing_windows as *mut _ as *mut c_void);
        //    XUngrabServer(self.display);
        //}

        loop {
            let mut event: XEvent = unsafe { std::mem::zeroed() };
            unsafe {
                XNextEvent(self.display, &mut event);
            }
            info!("Received event: {:#?}", event);

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

        if let Some(client) = self.clients.get(&(*event).window) {
            unsafe {
                XUnmapWindow(self.display, *client);
                XReparentWindow(self.display, (*event).window, self.root, 0, 0);
                XRemoveFromSaveSet(self.display, (*event).window);
                XDestroyWindow(self.display, *client);
                info!("Unframed client window: {:#?}", client);
            }
        } else {
            info!(
                "Ignoring UnmapNotify for non-client window: {:#?}",
                event.window
            );
            return;
        }

        self.clients.remove(&(*event).window);
    }

    fn on_button_press(&self, event: &XButtonEvent) {
        trace!("OnButtonPress event: {:#?}", *event);
    }

    fn on_map_request(&mut self, event: &XMapRequestEvent) {
        self.frame(&(*event).window, false);
        info!("OnMapRequest event: {:#?}", *event);
    }

    fn frame(&mut self, window: &Window, already_existing: bool) {
        let border_width: c_uint = 3;
        let border_color: c_ulong = 0xff0000;
        let bg_color: c_ulong = 0x0000ff;

        let window_attributes = unsafe {
            /* Safe because mem::zeroed is well defined here & panic on bad request*/
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
            return;
        }

        let frame = unsafe {
            XCreateSimpleWindow(
                self.display,
                self.root,
                window_attributes.x,
                window_attributes.y,
                window_attributes.width as c_uint,
                window_attributes.height as c_uint,
                border_width,
                border_color,
                bg_color,
            )
        };

        unsafe {
            XSelectInput(
                self.display,
                frame,
                SubstructureRedirectMask | SubstructureNotifyMask,
            );
            XAddToSaveSet(self.display, *window); /* offset */
            XReparentWindow(self.display, *window, frame, 0, 0);
            XMapWindow(self.display, frame);
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

        self.clients.insert(*window, frame);
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
        debug!("XWindowChanges: {:#?}", config);

        if let Some(frame) = self.clients.get(&(*event).window) {
            /* re-configure existing frame / decorations */
            unsafe {
                XConfigureWindow(self.display, *frame, event.value_mask as u32, &mut config);
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

    pub unsafe extern "C" fn on_wm_detected(
        _display: *mut Display,
        event: *mut XErrorEvent,
    ) -> c_int {
        assert_eq!(
            (*event).error_code,
            BadAccess,
            "Expected BadAccess error code OnWMDetected"
        );

        error!("Another window manager detected");

        let mut detected = WM_DETECTED.lock().unwrap();
        *detected = true;
        0 /* This is ignored */
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
