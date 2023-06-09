use std::net::{SocketAddr, UdpSocket};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::anyhow;

use crate::app::gates::Gates;
use crate::hal::gate::GateState;
use crate::svc::race_node::{FrameData, GateBeacon, RaceNode, RaceNodeMessage};
use crate::svc::CoordinatedInstant;

// This must be very strict (less than the acceptable error) because the application must switch
// to clock dead reckoning.
const COORDINATOR_BEACON_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Default, Debug)]
struct Stats {
    tx_count: usize,
    rx_count: usize,
}

pub struct StdRaceNode {
    #[allow(dead_code)]
    thread: Option<JoinHandle<Stats>>,
    state: SharedNodeState,
    continue_running: Arc<AtomicBool>,
    // Note: not using mpsc because it causes weird bugs (maybe esp-idf implementation is buggy)
    tx: Arc<Mutex<Option<RaceNodeMessage>>>,
}

struct StdRaceNodeConfig {
    sender_addr: SocketAddr,
    receiver_addr: SocketAddr,
    broadcast_addr: SocketAddr,
}

impl Default for StdRaceNodeConfig {
    fn default() -> Self {
        Self {
            sender_addr: "0.0.0.0:0".parse().unwrap(),
            receiver_addr: "0.0.0.0:6699".parse().unwrap(),
            broadcast_addr: "255.255.255.255:6699".parse().unwrap(),
        }
    }
}

impl StdRaceNode {
    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_config(StdRaceNodeConfig::default())
    }

    fn new_with_config(config: StdRaceNodeConfig) -> anyhow::Result<Self> {
        let StdRaceNodeConfig {
            sender_addr,
            receiver_addr,
            broadcast_addr,
        } = config;

        let state = SharedNodeState::default();

        log::info!("Starting race node");

        let sender = make_sender(sender_addr)?;
        let receiver = make_receiver(receiver_addr)?;

        let continue_running = Arc::new(AtomicBool::new(true));

        let (thread, tx) = spawn_thread(
            broadcast_addr,
            state.clone(),
            sender,
            receiver,
            continue_running.clone(),
        );

        Ok(StdRaceNode {
            thread: Some(thread),
            state,
            continue_running,
            tx,
        })
    }

    fn stop(&mut self) -> Option<Stats> {
        self.continue_running.store(false, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            thread.join().ok()
        } else {
            None
        }
    }
}

impl Drop for StdRaceNode {
    fn drop(&mut self) {
        if let Some(stats) = self.stop() {
            log::debug!("stats: {:?}", stats);
        }
    }
}

fn spawn_thread(
    broadcast_addr: SocketAddr,
    state: SharedNodeState,
    sender: UdpSocket,
    mut receiver: UdpSocket,
    continue_running: Arc<AtomicBool>,
) -> (JoinHandle<Stats>, Arc<Mutex<Option<RaceNodeMessage>>>) {
    const TASK_WAKEUP_PERIOD: Duration = Duration::from_millis(20);

    let tx_message: Option<RaceNodeMessage> = None;
    let tx = Arc::new(Mutex::new(tx_message));
    let tx_copy = tx.clone();

    let thread = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            let mut stats = Stats::default();

            loop {
                let start = Instant::now();

                // log::info!("node update");

                let next_wakeup = Instant::now() + TASK_WAKEUP_PERIOD;

                if let Some(tx_msg) = tx_copy.try_lock().ok().and_then(|x| x.clone()) {
                    if sender
                        .send_to(tx_msg.data().as_bytes(), broadcast_addr)
                        .is_ok()
                    {
                        stats.tx_count += 1;
                    }
                }

                // receive_message should lock for the timeout set to the udp socket
                while let Ok(rx_msg) = receive_message(&mut receiver) {
                    log::debug!("{:?}", rx_msg);
                    stats.rx_count += 1;

                    match rx_msg {
                        RaceNodeMessage::GateBeacon(beacon) => state.try_modify(|x| {
                            update_gate(&mut x.gates, &beacon, x.coordinator_time.into_option())
                        }),
                        RaceNodeMessage::CoordinatorBeacon(beacon) => state.try_modify(|x| {
                            x.coordinator_time = ExpOpt::<CoordinatedInstant>::new_with_duration(
                                beacon.time,
                                COORDINATOR_BEACON_TIMEOUT,
                            );
                            x.coordinator_beacon_time = Some(Instant::now());
                        }),
                    }
                }

                if !continue_running.load(Ordering::Acquire) {
                    break;
                }

                log::trace!(
                    "node update took {}ms",
                    (Instant::now() - start).as_millis()
                );

                // Ensure this task is not spinning
                if let Some(delay) = next_wakeup.checked_duration_since(Instant::now()) {
                    sleep(delay);
                } else {
                    log::error!("no delay");
                }
            }
            stats
        })
        .unwrap();

    (thread, tx)
}

fn make_receiver(receiver_addr: SocketAddr) -> anyhow::Result<UdpSocket> {
    let receiver = UdpSocket::bind(receiver_addr)?;
    receiver.set_broadcast(true)?;

    // This must be non blocking, otherwise the thread may be locked.
    // It is not important that all messages are successfully sent.
    receiver.set_nonblocking(true)?;

    log::info!("receiver {:?}", receiver.local_addr());
    Ok(receiver)
}

fn make_sender(sender_addr: SocketAddr) -> anyhow::Result<UdpSocket> {
    let sender = UdpSocket::bind(sender_addr)?;
    sender.set_broadcast(true)?;

    // This must be non blocking, otherwise the thread may be locked.
    // It is not important that all messages are successfully sent.
    sender.set_nonblocking(true)?;

    log::info!("sender {:?}", sender.local_addr());
    Ok(sender)
}

fn receive_message(receiver: &mut UdpSocket) -> anyhow::Result<RaceNodeMessage> {
    let mut buf = [0u8; RaceNodeMessage::FRAME_SIZE];

    if let Ok((number_of_bytes, _src_addr)) = receiver.recv_from(&mut buf) {
        if number_of_bytes == RaceNodeMessage::FRAME_SIZE {
            let data = FrameData::from(buf);
            RaceNodeMessage::try_from(data).map_err(|_| anyhow!("Cannot parse"))
        } else {
            Err(anyhow!("Wrong number of bytes"))
        }
    } else {
        Err(anyhow!("No messages"))
    }
}

impl RaceNode for StdRaceNode {
    fn set_coordinator_time(&self, t: CoordinatedInstant) {
        self.state.try_modify(|x| {
            // This timeout must be very strict, because set_coordinator_time is
            // called when the node is a coordinator.
            const TIMEOUT: Duration = Duration::from_millis(100);

            if let Some(expiration) = x.coordinator_time.expiration {
                let now = Instant::now();
                if now > expiration {
                    log::warn!("set_coordinator_time called too late");
                }
            }

            x.coordinator_time = ExpOpt::<CoordinatedInstant>::new_with_duration(t, TIMEOUT)
        })
    }

    fn coordinator_time(&self) -> Option<CoordinatedInstant> {
        self.state
            .read(|x| x.coordinator_time.into_option())
            .flatten()
    }

    fn publish(&self, msg: RaceNodeMessage) -> anyhow::Result<()> {
        self.tx
            .try_lock()
            .map(|mut x| *x = Some(msg))
            .map_err(|_| anyhow!("Cannot publish"))
    }

    fn gates(&self) -> Gates {
        self.state.read(|x| x.gates.to_owned()).unwrap()
    }

    fn time_since_coordinator_beacon(&self) -> Duration {
        self.state
            .read(|x| x.coordinator_beacon_time)
            .flatten()
            .and_then(|instant| Instant::now().checked_duration_since(instant))
            .unwrap_or(Duration::MAX)
    }
}

#[derive(Default)]
struct NodesState {
    coordinator_time: ExpOpt<CoordinatedInstant>,
    coordinator_beacon_time: Option<Instant>,
    gates: Gates,
}

#[derive(Clone)]
struct SharedNodeState(Arc<Mutex<NodesState>>);

impl Default for SharedNodeState {
    fn default() -> Self {
        SharedNodeState(Arc::new(Mutex::new(NodesState::default())))
    }
}

impl SharedNodeState {
    fn try_modify<F>(&self, f: F)
    where
        F: FnOnce(&mut NodesState),
    {
        // Ignore errors
        self.0.try_lock().map(|mut x| f(x.deref_mut())).ok();
    }

    fn read<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&NodesState) -> T,
    {
        self.0.lock().map(|x| f(x.deref())).ok()
    }
}

fn update_gate(gates: &mut Gates, gate: &GateBeacon, coordinated_time: Option<CoordinatedInstant>) {
    let &GateBeacon {
        addr,
        state,
        last_activation_time,
    } = gate;
    if let Some(gate) = gates.get_mut_from_addr(addr) {
        gate.active = state == GateState::Active;
        gate.last_activation_time = last_activation_time;
        gate.last_beacon_time = coordinated_time;
    }
}

#[derive(Default, Copy, Clone)]
struct ExpOpt<T> {
    value: Option<T>,
    expiration: Option<std::time::Instant>,
}

impl<T> ExpOpt<T> {
    fn new_with_expiration(value: T, expiration: std::time::Instant) -> Self {
        Self {
            value: Some(value),
            expiration: Some(expiration),
        }
    }
    fn new_with_duration(value: T, duration: std::time::Duration) -> Self {
        Self::new_with_expiration(value, std::time::Instant::now() + duration)
    }

    fn into_option(self) -> Option<T> {
        let now = std::time::Instant::now();

        let expired = if let Some(expiration) = self.expiration {
            expiration < now
        } else {
            false
        };

        if expired {
            log::trace!(
                "Expired {}ms ago",
                now.duration_since(self.expiration.unwrap()).as_millis()
            );
            None
        } else {
            self.value
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::svc::race_node::{CoordinatorBeacon, RaceNode};
    use crate::svc::std_race_node::StdRaceNodeConfig;
    use crate::svc::{CoordinatedInstant, StdRaceNode};

    fn make_coordinator_node() -> StdRaceNode {
        // Broadcast does not work on localhost, so we just use different ports

        let cfg = StdRaceNodeConfig {
            sender_addr: "0.0.0.0:0".parse().unwrap(),
            receiver_addr: "127.0.0.10:6699".parse().unwrap(),
            broadcast_addr: "127.0.0.10:6698".parse().unwrap(),
        };

        StdRaceNode::new_with_config(cfg).unwrap()
    }

    fn make_start_node() -> StdRaceNode {
        // Broadcast does not work on localhost, so we just use different ports

        let cfg = StdRaceNodeConfig {
            sender_addr: "0.0.0.0:0".parse().unwrap(),
            receiver_addr: "127.0.0.10:6698".parse().unwrap(),
            broadcast_addr: "127.0.0.10:6699".parse().unwrap(),
        };

        StdRaceNode::new_with_config(cfg).unwrap()
    }

    #[ignore]
    #[test_log::test]
    fn test_two_nodes_can_talk() {
        log::info!("test_two_nodes_can_talk");
        let mut coordinator_node = make_coordinator_node();
        let mut start_node = make_start_node();

        coordinator_node
            .publish(
                CoordinatorBeacon {
                    time: CoordinatedInstant::from_millis(123),
                }
                .into(),
            )
            .unwrap();

        std::thread::sleep(Duration::from_secs(1));

        let coordinator_stats = coordinator_node.stop().unwrap();
        let start_stats = start_node.stop().unwrap();

        assert_eq!(coordinator_stats.rx_count, 0);
        assert!(coordinator_stats.tx_count > 0);

        assert!(start_stats.rx_count > 0);
        assert_eq!(start_stats.tx_count, 0);
    }
}
