pub trait Button {
    fn is_pressed(&self) -> bool {
        self.state() == ButtonState::Pressed
    }
    fn state(&self) -> ButtonState;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum ButtonState {
    #[default]
    Released,
    Pressed,
}
