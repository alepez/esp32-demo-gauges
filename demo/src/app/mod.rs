use std::time::{Duration, Instant};

pub use crate::app::gates::Gate;
pub use crate::app::gates::Gates;
pub use crate::app::race::Race;

use crate::hal::button::ButtonState;
use crate::hal::gate::GateState;
use crate::hal::rgb_led::RgbLed;
use crate::hal::rgb_led::RgbLedColor;
use crate::hal::Platform;
use crate::svc::race_node::*;
use crate::svc::{
    calculate_clock_offset, CoordinatedClock, CoordinatedInstant, LocalClock, LocalInstant,
};

pub mod gates;
mod race;

#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemState {
    pub time: CoordinatedInstant,
    pub gates: Gates,
    pub race: Race,
}

struct Services<'a> {
    led_controller: LedController<'a>,
    platform: &'a dyn Platform,
    local_clock: LocalClock,
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum AppState {
    Init(InitState),
    CoordinatorReady(CoordinatorReadyState),
    GateStartup(GateStartupState),
    GateReady(GateReadyState),
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Init(InitState::default())
    }
}

pub struct App<'a> {
    services: Services<'a>,
    state: AppState,
}

impl<'a> App<'a> {
    pub fn new(platform: &'a mut (dyn Platform)) -> Self {
        let led_controller = LedController {
            led: platform.rgb_led(),
        };

        let race_clock = LocalClock::default();

        let services = Services {
            led_controller,
            platform,
            local_clock: race_clock,
        };

        let state = AppState::default();

        Self { services, state }
    }

    pub fn update(&mut self) {
        let new_state = match &mut self.state {
            AppState::Init(state) => state.update(&self.services),
            AppState::CoordinatorReady(state) => state.update(&self.services),
            AppState::GateStartup(state) => state.update(&self.services),
            AppState::GateReady(state) => state.update(&self.services),
        };

        if new_state != self.state {
            // log::info!("{:?}", &new_state);
            self.state = new_state;
        }

        self.services.led_controller.update(&self.state);
    }
}

struct LedController<'a> {
    led: &'a dyn RgbLed,
}

impl<'a> LedController<'a> {
    pub fn update(&mut self, app_state: &AppState) {
        let color = color_from_app_state(app_state);
        self.led.set_color(RgbLedColor::from(color));
    }
}

fn color_from_app_state(app_state: &AppState) -> u32 {
    const RED: u32 = 0xFF0000;
    const YELLOW: u32 = 0xFFFF00;
    const GREEN: u32 = 0x00FF00;
    const BLUE: u32 = 0x0000FF;
    const WHITE: u32 = 0xFFFFFF;

    match app_state {
        AppState::Init(_) => RED,
        AppState::GateStartup(_) => YELLOW,
        AppState::CoordinatorReady(state) => {
            if state.any_gate_active {
                BLUE
            } else {
                WHITE
            }
        }
        AppState::GateReady(state) => {
            if state.gate_state == GateState::Active {
                BLUE
            } else {
                GREEN
            }
        }
    }
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
struct InitState {
    gate_state: GateState,
    button_state: ButtonState,
    time: LocalInstant,
}

impl InitState {
    pub fn update(&mut self, services: &Services) -> AppState {
        let gate_state = services.platform.gate().state();
        let button_state = services.platform.button().state();
        let local_time = services.local_clock.now().expect("Cannot get time");
        let address = address(services);

        let startup_as_gate = address.is_gate()
            && (button_state != ButtonState::Pressed)
            && (gate_state != GateState::Active);

        /* Coordinator is selected by dip switch */
        let startup_as_coordinator = address.is_coordinator()
            && (button_state != ButtonState::Pressed)
            && (gate_state != GateState::Active);

        if startup_as_gate {
            log::info!("This is a gate");
            AppState::GateStartup(GateStartupState::default())
        } else if startup_as_coordinator {
            log::info!("This is a coordinator");
            // On coordinator, local time is the coordinated time, without any offset
            AppState::CoordinatorReady(CoordinatorReadyState {
                time: CoordinatedInstant::from_millis(local_time.as_millis()),
                system_state: SystemState::default(),
                any_gate_active: false,
            })
        } else {
            AppState::Init(*self)
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq, Debug)]
struct CoordinatorReadyState {
    time: CoordinatedInstant,
    system_state: SystemState,
    any_gate_active: bool,
}

impl CoordinatorReadyState {
    pub fn update(&self, services: &Services) -> AppState {
        let is_wifi_connected = services.platform.wifi().is_up();

        if !is_wifi_connected {
            // If coordinator looses connection, the system is not reliable and
            // we must start again.
            return AppState::Init(InitState::default());
        }

        let local_time = services.local_clock.now().expect("Cannot get time");

        // On coordinator, local time is the coordinated time, without any offset
        let time = CoordinatedInstant::from_millis(local_time.as_millis());

        let beacon = CoordinatorBeacon { time };

        if let Err(e) = services.platform.race_node().publish(beacon.into()) {
            log::error!("{e}");
        }

        services.platform.race_node().set_coordinator_time(time);

        let gates = services.platform.race_node().gates();

        let mut race = self.system_state.race.clone();

        race.set_gates(&gates);

        let any_gate_active = gates.start_gate().active || gates.finish_gate().active;

        let system_state = SystemState { time, gates, race };

        services
            .platform
            .http_server()
            .set_system_state(&self.system_state);

        AppState::CoordinatorReady(CoordinatorReadyState {
            time,
            system_state,
            any_gate_active,
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct GateStartupState {
    time_started: Instant,
}

impl Default for GateStartupState {
    fn default() -> Self {
        Self {
            time_started: Instant::now(),
        }
    }
}

impl GateStartupState {
    pub fn update(&mut self, services: &Services) -> AppState {
        let gate_state = services.platform.gate().state();

        if let Some(time_since_started) = Instant::now().checked_duration_since(self.time_started) {
            // Apparently, there's no way to recover the connection. Just panic and hope.
            const TIMEOUT: Duration = Duration::from_secs(10);
            if time_since_started > TIMEOUT {
                panic!();
            }
        }

        if let Some(coordinated_clock) = make_coordinated_clock(services) {
            AppState::GateReady(GateReadyState {
                gate_state,
                coordinated_clock,
                last_activation_time: None,
            })
        } else {
            AppState::GateStartup(*self)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct GateReadyState {
    gate_state: GateState,
    coordinated_clock: CoordinatedClock,
    last_activation_time: Option<CoordinatedInstant>,
}

impl GateReadyState {
    pub fn update(&mut self, services: &Services) -> AppState {
        let gate_state = services.platform.gate().state();
        let button_state = services.platform.button().state();

        let coordinated_clock = make_coordinated_clock(services).unwrap_or(self.coordinated_clock);
        let coordinated_time = coordinated_clock.now();

        if services
            .platform
            .race_node()
            .time_since_coordinator_beacon()
            > Duration::from_secs(10)
        {
            // No beacon from coordinator, maybe due to a disconnection
            return AppState::GateStartup(GateStartupState::default());
        }

        log::trace!("coordinated_time: {}", coordinated_time.as_millis());

        let addr = address(services);

        // Button is useful for testing. So we assume a button pressed event
        // like a gate active event.
        let gate_state = gate_state_or_button(gate_state, button_state);

        let last_activation_time = if gate_state == GateState::Active {
            Some(coordinated_time)
        } else {
            self.last_activation_time
        };

        let beacon = GateBeacon {
            addr,
            state: gate_state,
            last_activation_time,
        };

        if let Err(e) = services.platform.race_node().publish(beacon.into()) {
            log::error!("{e}");
        }

        AppState::GateReady(GateReadyState {
            gate_state,
            coordinated_clock,
            last_activation_time,
        })
    }
}

fn address(services: &Services) -> NodeAddress {
    address_from_env_var().unwrap_or_else(|| services.platform.dip_switch().address())
}

fn address_from_env_var() -> Option<NodeAddress> {
    option_env!("RACEGATE_NODE_ADDRESS")?
        .parse::<u8>()
        .ok()
        .map(NodeAddress::from)
}

fn make_coordinated_clock(services: &Services) -> Option<CoordinatedClock> {
    let time = services.local_clock.now()?;

    services
        .platform
        .race_node()
        .coordinator_time()
        .map(|coord_time| calculate_clock_offset(coord_time, time))
        .map(|clock_offset| CoordinatedClock::new(services.local_clock, clock_offset))
}

fn gate_state_or_button(gate: GateState, button: ButtonState) -> GateState {
    if gate == GateState::Active || button == ButtonState::Pressed {
        GateState::Active
    } else {
        GateState::Inactive
    }
}
