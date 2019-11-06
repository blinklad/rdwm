#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use env_logger::WriteStyle::Auto;
use libc::*;
use std::sync::Mutex;
use x11::xlib::*;

lazy_static! {
    static ref WM_DETECTED: Mutex<bool> = Mutex::new(false);
}

#[derive(Debug)]
struct Rdwm {
    display: *mut Display,
    root: Window,
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

        debug!("Display {:?} Root {:?}", display, root);
        info!("Display {:?} Root {:?}", display, root);
        Some(Rdwm { display, root })
    }

    fn run(&self) {
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
    }

    pub unsafe extern "C" fn on_wm_detected(
        _display: *mut Display,
        event: *mut XErrorEvent,
    ) -> c_int {
        assert_ne!((*event).error_code, BadAccess);

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
    let rdwm = Rdwm::init()
        .ok_or("could not connect to display server")
        .unwrap();
    info!("Starting display server OK");
    rdwm.run();

    info!("Finish OK");
    Ok(())
}
