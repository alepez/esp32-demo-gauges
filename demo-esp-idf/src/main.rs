use std::time::{Duration, Instant};

use demo::app::App;
use demo::hal::wifi::WifiConfig;
use esp_idf_sys as _;

use demo_esp_idf::platform::{BoardType, Config, PlatformImpl};

const TASK_WAKEUP_PERIOD: Duration = Duration::from_millis(20);

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let config = Config {
        wifi: WifiConfig::from_env_var().unwrap_or_default(),
        board_type: BoardType::RustDevKit,
    };

    log::info!("Create platform");
    let mut p = PlatformImpl::new(&config);

    log::info!("Create app");
    let mut app = App::new(&mut p);

    log::info!("Start loop");

    loop {
        let next_wakeup = Instant::now() + TASK_WAKEUP_PERIOD;

        {
            let start = Instant::now();
            app.update();

            log::trace!("app update took {}ms", (Instant::now() - start).as_millis());
        }

        if let Some(delay) = next_wakeup.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay);
        } else {
            log::error!("no delay");
        }
    }
}
