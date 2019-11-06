#[macro_use]
extern crate log;
use env_logger::WriteStyle::Auto;
use x11::xlib::*;

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
        let _wm_detected = false;
        unsafe {
            /* Safe because TODO */
            XSetErrorHandler(Some(Rdwm::on_wm_detected));
        }
    }

    pub unsafe extern "C" fn on_wm_detected(
        _display: *mut Display,
        event: *mut XErrorEvent,
    ) -> libc::c_int {
        assert_ne!((*event).error_code, BadAccess);
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
