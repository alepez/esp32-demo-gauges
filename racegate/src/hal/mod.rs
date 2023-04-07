use crate::hal::button::Button;
use crate::hal::gate::Gate;
use crate::hal::rgb_led::RgbLed;
use crate::hal::wifi::Wifi;
use crate::svc::HttpServer;

pub mod button;
pub mod gate;
pub mod rgb_led;
pub mod wifi;

pub trait Platform {
    fn button(&self) -> &(dyn Button + '_);
    fn gate(&self) -> &(dyn Gate + '_);
    fn http_server(&self) -> &(dyn HttpServer + '_);
    fn rgb_led(&self) -> &(dyn RgbLed + '_);
    fn wifi(&self) -> &(dyn Wifi + '_);
}
