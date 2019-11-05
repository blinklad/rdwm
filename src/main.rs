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
        unsafe {
            let mut display = XOpenDisplay(std::ptr::null());
            let root = XDefaultRootWindow(display);
            debug!("Display {:?} Root {:?}", display, root);
            Some(Rdwm { display, root })
        }
    }

    fn _run(&self) {
        unimplemented!();
    }
}

impl Drop for Rdwm {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .write_style(Auto)
        .init();
    info!("Starting");
    let rdwm: Rdwm = Rdwm::init()
        .ok_or("could not connect to display server")
        .unwrap();

    info!("Finish OK");
    Ok(())
}
