#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate bitflags;

mod rdwm;
use env_logger::WriteStyle::Auto;
use rdwm::Rdwm;

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
