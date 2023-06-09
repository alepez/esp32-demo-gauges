use demo::hal::button::Button;
use demo::hal::rgb_led::RgbLed;
use demo::hal::wifi::{Wifi, WifiConfig};
use demo::hal::Platform;
use demo::svc::HttpServer;
use esp_idf_hal::gpio::InputPin;
use esp_idf_hal::peripherals::Peripherals;

use crate::drivers::button::EspButton;
use crate::drivers::http::HttpServer as EspHttpServer;
use crate::drivers::rgb_led::WS2812RgbLed;
use crate::drivers::wifi::EspWifi;

pub enum BoardType {
    M5StampC3,
    RustDevKit,
}

pub struct PlatformImpl {
    wifi: EspWifi,
    rgb_led: WS2812RgbLed,
    button: EspButton,
    http_server: EspHttpServer,
}

pub struct Config {
    pub wifi: WifiConfig<'static>,
    pub board_type: BoardType,
}

impl PlatformImpl {
    pub fn new(config: &Config) -> Self {
        let peripherals = Peripherals::take().unwrap();

        let wifi = EspWifi::new(peripherals.modem).expect("Cannot create Wi-Fi");
        wifi.setup(&config.wifi).expect("Cannot setup Wi-Fi");

        let rgb_led = WS2812RgbLed::default();

        let button_pin = match config.board_type {
            BoardType::M5StampC3 => peripherals.pins.gpio3.downgrade_input(),
            BoardType::RustDevKit => peripherals.pins.gpio9.downgrade_input(),
        };

        let button = EspButton::new(button_pin).expect("Cannot setup button");
        let http_server = EspHttpServer::new().expect("Cannot setup http server");

        Self {
            wifi,
            rgb_led,
            button,
            http_server,
        }
    }
}

impl Platform for PlatformImpl {
    fn wifi(&self) -> &(dyn Wifi + '_) {
        &self.wifi
    }

    fn rgb_led(&self) -> &(dyn RgbLed + '_) {
        &self.rgb_led
    }

    fn button(&self) -> &(dyn Button + '_) {
        &self.button
    }

    fn http_server(&self) -> &(dyn HttpServer + '_) {
        &self.http_server
    }
}
