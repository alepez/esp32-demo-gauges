use crate::svc::race_node::NodeAddress;

pub trait DipSwitch {
    fn address(&self) -> NodeAddress;
}
