use crate::svc::race_node::NodeAddress;
use crate::svc::CoordinatedInstant;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, Clone, Eq, PartialEq)]
pub struct Gate {
    pub active: bool,
    pub last_activation_time: Option<CoordinatedInstant>,
    pub last_beacon_time: Option<CoordinatedInstant>,
}

impl Gate {
    pub fn is_alive(&self, now: CoordinatedInstant) -> bool {
        self.last_beacon_time
            .map(|x| {
                let diff = now.as_millis() - x.as_millis();
                diff < 1_000
            })
            .unwrap_or(false)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, Clone, Eq, PartialEq)]
pub struct Gates {
    items: [Gate; 4],
}

impl Gates {
    pub fn new(items: [Gate; 4]) -> Self {
        Self { items }
    }

    pub fn start_gate(&self) -> &Gate {
        const INDEX: usize = NodeAddress::start().unwrap_as_gate_index();
        &self.items[INDEX]
    }

    pub fn finish_gate(&self) -> &Gate {
        const INDEX: usize = NodeAddress::finish().unwrap_as_gate_index();
        &self.items[INDEX]
    }

    pub fn get_mut_from_addr(&mut self, addr: NodeAddress) -> Option<&mut Gate> {
        let index = addr.as_gate_index()?;
        Some(&mut self.items[index])
    }
}
