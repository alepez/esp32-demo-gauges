use esp_idf_hal::gpio::{AnyInputPin, Input, PinDriver};
use racegate::hal::button::{Button, ButtonState};

pub struct EspButton {
    input: PinDriver<'static, AnyInputPin, Input>,
}

impl EspButton {
    pub fn new(pin: AnyInputPin) -> anyhow::Result<EspButton> {
        let input = PinDriver::input(pin)?;
        let input = input.into_input()?;
        Ok(Self { input })
    }
}

impl Button for EspButton {
    fn state(&self) -> ButtonState {
        if self.input.is_low() {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        }
    }
}
