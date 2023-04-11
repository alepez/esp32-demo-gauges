use crate::app::SystemState;
use crate::hal::gate::GateState::{Active, Inactive};
use crate::svc::clock::Instant;

pub enum Error {
    Unknown,
}

pub trait RaceNode {
    fn set_system_state(&self, status: &SystemState);

    fn set_node_address(&self, node_id: NodeAddress);

    fn coordinator(&self) -> Option<SystemState>;
}

#[derive(Debug, Copy, Clone)]
pub struct NodeAddress(u8);

impl NodeAddress {
    pub fn coordinator() -> Self {
        Self(0)
    }

    pub fn start() -> Self {
        Self(1)
    }

    pub fn finish() -> Self {
        Self(32)
    }

    pub fn is_coordinator(&self) -> bool {
        self.0 == 0
    }

    pub fn is_start(&self) -> bool {
        self.0 == 1
    }

    pub fn is_finish(&self) -> bool {
        self.0 == 32
    }
}

pub struct FrameData([u8; RaceNodeMessage::FRAME_SIZE]);

impl FrameData {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<[u8; 16]> for FrameData {
    fn from(value: [u8; 16]) -> Self {
        Self(value)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AddressedSystemState {
    pub addr: NodeAddress,
    pub state: SystemState,
}

#[derive(Debug)]
pub enum RaceNodeMessage {
    SystemState(AddressedSystemState),
}

impl RaceNodeMessage {
    pub const FRAME_SIZE: usize = 16;

    pub fn data(&self) -> FrameData {
        match self {
            RaceNodeMessage::SystemState(x) => x.into(),
        }
    }
}

impl TryFrom<FrameData> for RaceNodeMessage {
    type Error = Error;

    fn try_from(data: FrameData) -> Result<RaceNodeMessage, Error> {
        let msg_id = data.0.first().ok_or(Error::Unknown)?;
        match msg_id {
            1 => Ok(RaceNodeMessage::SystemState(
                AddressedSystemState::try_from(data)?,
            )),
            _ => Err(Error::Unknown),
        }
    }
}

impl TryFrom<FrameData> for AddressedSystemState {
    type Error = Error;

    fn try_from(data: FrameData) -> Result<AddressedSystemState, Error> {
        let addr = data.0.get(1).ok_or(Error::Unknown)?;
        let addr = NodeAddress(*addr);

        let gate_state = match data.0.get(2) {
            Some(0) => Inactive,
            Some(1) => Active,
            _ => Inactive,
        };

        let time = {
            let d0 = *data.0.get(3).ok_or(Error::Unknown)?;
            let d1 = *data.0.get(4).ok_or(Error::Unknown)?;
            let d2 = *data.0.get(5).ok_or(Error::Unknown)?;
            let d3 = *data.0.get(6).ok_or(Error::Unknown)?;
            let time_ms =
                ((d0 as u32) << 24) | ((d1 as u32) << 16) | ((d2 as u32) << 8) | (d3 as u32);
            Instant::from_millis(time_ms)
        };

        let state = SystemState { gate_state, time };

        Ok(AddressedSystemState { addr, state })
    }
}

fn serialize_system_state(x: &AddressedSystemState, data: &mut FrameData) {
    data.0[1] = x.addr.0;
    data.0[2] = x.state.gate_state as u8;
    data.0[3] = ((x.state.time.to_millis() >> 24) & 0xFF) as u8;
    data.0[4] = ((x.state.time.to_millis() >> 16) & 0xFF) as u8;
    data.0[5] = ((x.state.time.to_millis() >> 8) & 0xFF) as u8;
    data.0[6] = ((x.state.time.to_millis()) & 0xFF) as u8;
}

impl From<&AddressedSystemState> for FrameData {
    fn from(value: &AddressedSystemState) -> Self {
        FrameData::from(&RaceNodeMessage::SystemState(*value))
    }
}

impl From<&RaceNodeMessage> for FrameData {
    fn from(msg: &RaceNodeMessage) -> Self {
        let msg_id = match msg {
            RaceNodeMessage::SystemState(_) => 1,
        };

        let mut data = FrameData::from([msg_id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        match msg {
            RaceNodeMessage::SystemState(x) => serialize_system_state(x, &mut data),
        };

        data
    }
}

#[cfg(test)]
mod tests {
    use crate::hal::gate::GateState;

    use super::*;

    #[test]
    fn test_serialize_system_state() {
        let x = AddressedSystemState {
            addr: NodeAddress::start(),
            state: SystemState {
                gate_state: GateState::Active,
                time: Instant::from_millis(12345),
            },
        };

        let msg = RaceNodeMessage::SystemState(x);
        let data = msg.data();
        let data = data.as_bytes();

        insta::assert_debug_snapshot!(data);
    }
}
