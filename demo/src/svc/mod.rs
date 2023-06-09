use crate::app::SystemState;
pub use clock::{
    calculate_clock_offset, CoordinatedClock, CoordinatedInstant, LocalClock, LocalInstant,
    LocalOffset,
};
pub use race_node::RaceNode;
pub use std_race_node::StdRaceNode;

mod clock;
pub mod race_node;
mod std_race_node;

pub trait HttpServer {
    fn set_system_state(&self, status: &SystemState);
}
