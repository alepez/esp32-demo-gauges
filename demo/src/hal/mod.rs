use crate::hal::button::Button;
use crate::hal::dip_switch::DipSwitch;
use crate::hal::gate::Gate;
use crate::hal::rgb_led::RgbLed;
use crate::hal::wifi::Wifi;
use crate::svc::{race_node::RaceNode, HttpServer};

pub mod button;
pub mod dip_switch;
pub mod gate;
pub mod rgb_led;
pub mod wifi;

pub trait Platform {
    fn button(&self) -> &(dyn Button + '_);
    fn gate(&self) -> &(dyn Gate + '_);
    fn http_server(&self) -> &(dyn HttpServer + '_);
    fn race_node(&self) -> &(dyn RaceNode + '_);
    fn rgb_led(&self) -> &(dyn RgbLed + '_);
    fn wifi(&self) -> &(dyn Wifi + '_);
    fn dip_switch(&self) -> &(dyn DipSwitch + '_);
}
