use crate::app::gates::Gates;
use crate::hal::gate::GateState;
use crate::svc::CoordinatedInstant;
use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    Unknown,
}

pub trait RaceNode {
    fn set_coordinator_time(&self, t: CoordinatedInstant);

    fn coordinator_time(&self) -> Option<CoordinatedInstant>;

    fn publish(&self, msg: RaceNodeMessage) -> anyhow::Result<()>;

    fn gates(&self) -> Gates;

    fn time_since_coordinator_beacon(&self) -> Duration;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeAddress(u8);

const COORDINATOR_ADDRESS: NodeAddress = NodeAddress(0);
const START_ADDRESS: NodeAddress = NodeAddress(1);
const FINISH_ADDRESS: NodeAddress = NodeAddress(4);

impl NodeAddress {
    pub const fn coordinator() -> Self {
        COORDINATOR_ADDRESS
    }

    pub const fn start() -> Self {
        START_ADDRESS
    }

    pub const fn finish() -> Self {
        FINISH_ADDRESS
    }

    pub const fn is_coordinator(&self) -> bool {
        self.0 == COORDINATOR_ADDRESS.0
    }

    pub const fn is_gate(&self) -> bool {
        self.0 != COORDINATOR_ADDRESS.0
    }

    pub const fn is_start(&self) -> bool {
        self.0 == START_ADDRESS.0
    }

    pub const fn is_finish(&self) -> bool {
        self.0 == FINISH_ADDRESS.0
    }

    pub fn as_gate_index(&self) -> Option<usize> {
        if self.0 < 1 {
            None
        } else {
            Some((self.0 as usize) - 1)
        }
    }

    pub const fn unwrap_as_gate_index(&self) -> usize {
        if self.0 < 1 {
            panic!()
        } else {
            (self.0 as usize) - 1
        }
    }
}

impl From<u8> for NodeAddress {
    fn from(x: u8) -> Self {
        Self(x)
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
pub struct GateBeacon {
    pub addr: NodeAddress,
    pub state: GateState,
    pub last_activation_time: Option<CoordinatedInstant>,
}

#[derive(Debug, Copy, Clone)]
pub struct CoordinatorBeacon {
    pub time: CoordinatedInstant,
}

#[derive(Debug, Clone)]
pub enum RaceNodeMessage {
    GateBeacon(GateBeacon),
    CoordinatorBeacon(CoordinatorBeacon),
}

impl RaceNodeMessage {
    pub const FRAME_SIZE: usize = 16;

    pub fn data(&self) -> FrameData {
        FrameData::from(self)
    }
}

impl TryFrom<FrameData> for RaceNodeMessage {
    type Error = Error;

    fn try_from(data: FrameData) -> Result<RaceNodeMessage, Error> {
        let msg_id = data.0.first().ok_or(Error::Unknown)?;
        match msg_id {
            1 => Ok(GateBeacon::try_from(data)?.into()),
            2 => Ok(CoordinatorBeacon::try_from(data)?.into()),
            _ => Err(Error::Unknown),
        }
    }
}

impl TryFrom<FrameData> for GateBeacon {
    type Error = Error;

    fn try_from(data: FrameData) -> Result<GateBeacon, Error> {
        let addr = data.0.get(1).ok_or(Error::Unknown)?;
        let addr = NodeAddress(*addr);

        let gate_state = match data.0.get(2) {
            Some(0) => GateState::Inactive,
            Some(1) => GateState::Active,
            _ => GateState::Inactive,
        };

        let last_activation_time = deserialize_u32(&data, 3).ok_or(Error::Unknown)?;

        let last_activation_time = if last_activation_time == 0 {
            None
        } else {
            Some(CoordinatedInstant::from_millis(last_activation_time as i32))
        };

        Ok(GateBeacon {
            addr,
            state: gate_state,
            last_activation_time,
        })
    }
}

impl TryFrom<FrameData> for CoordinatorBeacon {
    type Error = Error;

    fn try_from(data: FrameData) -> Result<CoordinatorBeacon, Error> {
        let time = CoordinatedInstant::from_millis(
            deserialize_u32(&data, 1).ok_or(Error::Unknown)? as i32,
        );

        Ok(CoordinatorBeacon { time })
    }
}

impl From<CoordinatorBeacon> for RaceNodeMessage {
    fn from(x: CoordinatorBeacon) -> Self {
        RaceNodeMessage::CoordinatorBeacon(x)
    }
}

impl From<GateBeacon> for RaceNodeMessage {
    fn from(x: GateBeacon) -> Self {
        RaceNodeMessage::GateBeacon(x)
    }
}

fn serialize_system_state(x: &GateBeacon, data: &mut FrameData) {
    data.0[1] = x.addr.0;
    data.0[2] = x.state as u8;

    if let Some(last_activation_time) = x.last_activation_time {
        serialize_u32(last_activation_time.as_millis() as u32, data, 3);
    } else {
        serialize_u32(0, data, 3);
    }
}

fn serialize_coordinator_beacon(x: &CoordinatorBeacon, data: &mut FrameData) {
    serialize_u32(x.time.as_millis() as u32, data, 1);
}

fn serialize_msg_id(msg: &RaceNodeMessage, data: &mut FrameData) {
    let msg_id = match msg {
        RaceNodeMessage::GateBeacon(_) => 1,
        RaceNodeMessage::CoordinatorBeacon(_) => 2,
    };

    data.0[0] = msg_id;
}

fn serialize_u32(x: u32, data: &mut FrameData, offset: usize) {
    data.0[offset] = ((x >> 24) & 0xFF) as u8;
    data.0[offset + 1] = ((x >> 16) & 0xFF) as u8;
    data.0[offset + 2] = ((x >> 8) & 0xFF) as u8;
    data.0[offset + 3] = ((x) & 0xFF) as u8;
}

fn deserialize_u32(data: &FrameData, offset: usize) -> Option<u32> {
    let d0 = *data.0.get(offset)?;
    let d1 = *data.0.get(offset + 1)?;
    let d2 = *data.0.get(offset + 2)?;
    let d3 = *data.0.get(offset + 3)?;
    Some(((d0 as u32) << 24) | ((d1 as u32) << 16) | ((d2 as u32) << 8) | (d3 as u32))
}

impl From<&RaceNodeMessage> for FrameData {
    fn from(msg: &RaceNodeMessage) -> Self {
        let mut data = FrameData::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        serialize_msg_id(msg, &mut data);

        match msg {
            RaceNodeMessage::GateBeacon(x) => serialize_system_state(x, &mut data),
            RaceNodeMessage::CoordinatorBeacon(x) => serialize_coordinator_beacon(x, &mut data),
        };

        data
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use crate::hal::gate::GateState;

    use super::*;

    #[test]
    fn test_serialize_system_state() {
        let x = GateBeacon {
            addr: NodeAddress::start(),
            state: GateState::Active,
            last_activation_time: Some(CoordinatedInstant::from_millis(12345)),
        };

        let msg = RaceNodeMessage::GateBeacon(x);
        let data = msg.data();

        assert_debug_snapshot!(data.as_bytes());
        assert_debug_snapshot!(RaceNodeMessage::try_from(data).unwrap());
    }

    #[test]
    fn test_serialize_coordinator_beacon() {
        let x = CoordinatorBeacon {
            time: CoordinatedInstant::from_millis(2_123_456_789),
        };

        let msg = RaceNodeMessage::CoordinatorBeacon(x);
        let data = msg.data();

        assert_debug_snapshot!(data.as_bytes());
        assert_debug_snapshot!(RaceNodeMessage::try_from(data).unwrap());
    }
}
