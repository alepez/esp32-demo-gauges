use esp_idf_hal::gpio::{AnyInputPin, Input, PinDriver};
use racegate::hal::dip_switch::DipSwitch;
use racegate::svc::race_node::NodeAddress;

pub struct EspDipSwitch {
    pins: [PinDriver<'static, AnyInputPin, Input>; 3],
}

fn pin_to_driver(pin: AnyInputPin) -> PinDriver<'static, AnyInputPin, Input> {
    PinDriver::input(pin).unwrap().into_input().unwrap()
}

impl EspDipSwitch {
    pub fn new(pins: [AnyInputPin; 3]) -> anyhow::Result<EspDipSwitch> {
        let [p0, p1, p2] = pins;
        Ok(Self {
            pins: [pin_to_driver(p0), pin_to_driver(p1), pin_to_driver(p2)],
        })
    }
}

impl DipSwitch for EspDipSwitch {
    fn address(&self) -> NodeAddress {
        let p0 = self.pins[0].is_low() as u8;
        let p1 = self.pins[1].is_low() as u8;
        let p2 = self.pins[2].is_low() as u8;

        NodeAddress::from((p2 << 2) | (p1 << 1) | p0)
    }
}
