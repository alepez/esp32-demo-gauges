pub trait Gate {
    fn is_active(&self) -> bool {
        self.state() == GateState::Active
    }

    fn state(&self) -> GateState;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum GateState {
    #[default]
    Inactive,
    Active,
}
