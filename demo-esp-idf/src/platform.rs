use esp_idf_hal::gpio::InputPin;
use esp_idf_hal::peripherals::Peripherals;
use racegate::hal::button::Button;
use racegate::hal::dip_switch::DipSwitch;
use racegate::hal::gate::Gate;
use racegate::hal::rgb_led::RgbLed;
use racegate::hal::wifi::{Wifi, WifiConfig};
use racegate::hal::Platform;
use racegate::svc::{HttpServer, RaceNode};

use crate::drivers::button::EspButton;
use crate::drivers::dip_switch::EspDipSwitch;
use crate::drivers::gate::EspGate;
use crate::drivers::http::HttpServer as EspHttpServer;
use crate::drivers::race_node::EspRaceNode;
use crate::drivers::rgb_led::WS2812RgbLed;
use crate::drivers::wifi::EspWifi;

pub enum BoardType {
    M5StampC3,
    RustDevKit,
}

pub struct PlatformImpl {
    wifi: EspWifi,
    rgb_led: WS2812RgbLed,
    gate: EspGate,
    button: EspButton,
    http_server: EspHttpServer,
    race_node: EspRaceNode,
    dip_switch: EspDipSwitch,
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

        let gate_pin = match config.board_type {
            BoardType::M5StampC3 => peripherals.pins.gpio4.downgrade_input(),
            BoardType::RustDevKit => peripherals.pins.gpio8.downgrade_input(),
        };

        let button_pin = match config.board_type {
            BoardType::M5StampC3 => peripherals.pins.gpio3.downgrade_input(),
            BoardType::RustDevKit => peripherals.pins.gpio9.downgrade_input(),
        };

        let dip_switch_pins = [
            peripherals.pins.gpio5.downgrade_input(),
            peripherals.pins.gpio6.downgrade_input(),
            peripherals.pins.gpio7.downgrade_input(),
        ];

        let gate = EspGate::new(gate_pin).expect("Cannot setup gate");
        let button = EspButton::new(button_pin).expect("Cannot setup button");
        let http_server = EspHttpServer::new().expect("Cannot setup http server");
        let race_node = EspRaceNode::new().expect("Cannot setup race node");
        let dip_switch = EspDipSwitch::new(dip_switch_pins).expect("Cannot setup dip switch");

        Self {
            wifi,
            rgb_led,
            gate,
            button,
            http_server,
            race_node,
            dip_switch,
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

    fn gate(&self) -> &(dyn Gate + '_) {
        &self.gate
    }

    fn button(&self) -> &(dyn Button + '_) {
        &self.button
    }

    fn http_server(&self) -> &(dyn HttpServer + '_) {
        &self.http_server
    }

    fn race_node(&self) -> &(dyn RaceNode + '_) {
        &self.race_node
    }

    fn dip_switch(&self) -> &(dyn DipSwitch + '_) {
        &self.dip_switch
    }
}
