use std::time::Duration;

use esp_idf_sys as _;

use racegate::app::App;
use racegate::config::Config;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let config = Config::default();

    log::info!("Create platform");
    let mut p = racegate::platform::create(&config);
    let p = p.as_mut();

    log::info!("Create app");
    let mut app = App::new(p);

    let period = Duration::from_millis(20);

    log::info!("Start loop");
    loop {
        std::thread::sleep(period);
        app.update();
    }
}
