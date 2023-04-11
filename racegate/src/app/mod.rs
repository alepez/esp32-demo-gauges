use std::time::Instant;

use crate::hal::button::ButtonState;
use crate::hal::gate::GateState;
use crate::hal::rgb_led::RgbLed;
use crate::hal::rgb_led::RgbLedColor;
use crate::hal::Platform;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct SystemState {
    pub gate_state: GateState,
    pub time: RaceInstant,
}

impl From<&AppState> for SystemState {
    fn from(value: &AppState) -> Self {
        match value {
            AppState::Init(x) => SystemState {
                gate_state: GateState::Inactive,
                time: x.time,
            },
            AppState::Ready(x) => SystemState {
                gate_state: x.gate_state,
                time: x.time,
            },
        }
    }
}

struct Services<'a> {
    led_controller: LedController<'a>,
    platform: &'a dyn Platform,
    race_clock: RaceClock,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum AppState {
    Init(InitAppState),
    Ready(ReadyAppState),
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Init(InitAppState::default())
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

        let race_clock = RaceClock::new();

        let services = Services {
            led_controller,
            platform,
            race_clock,
        };

        let state = AppState::default();

        Self { services, state }
    }

    pub fn update(&mut self) {
        let new_state = match self.state {
            AppState::Init(mut state) => state.update(&self.services),
            AppState::Ready(mut state) => state.update(&self.services),
        };

        if new_state != self.state {
            log::info!("{:?}", &new_state);
            self.state = new_state;
        }

        self.services.led_controller.update(&self.state);

        let system_state = (&self.state).into();

        self.services
            .platform
            .http_server()
            .set_system_state(&system_state);

        self.services
            .platform
            .race_node()
            .set_system_state(&system_state);
    }
}

struct LedController<'a> {
    led: &'a dyn RgbLed,
}

impl<'a> LedController<'a> {
    pub fn update(&mut self, app_state: &AppState) {
        let color = match app_state {
            AppState::Init(_) => 0xFF0000,
            AppState::Ready(state) => {
                if state.gate_state == GateState::Active {
                    0x008080
                } else if state.is_wifi_connected {
                    0x008000
                } else {
                    0x800000
                }
            }
        };

        self.led.set_color(RgbLedColor::from(color));
    }
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
struct InitAppState {
    is_wifi_connected: bool,
    gate_state: GateState,
    button_state: ButtonState,
    time: RaceInstant,
}

impl InitAppState {
    pub fn update(&mut self, services: &Services) -> AppState {
        let is_wifi_connected = services.platform.wifi().is_connected();
        let gate_state = services.platform.gate().state();
        let button_state = services.platform.button().state();
        let time = services.race_clock.now().expect("Cannot get time");

        if is_wifi_connected
            && (button_state != ButtonState::Pressed)
            && gate_state != GateState::Active
        {
            AppState::Ready(ReadyAppState {
                is_wifi_connected,
                gate_state,
                button_state,
                time,
            })
        } else {
            AppState::Init(*self)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct ReadyAppState {
    is_wifi_connected: bool,
    gate_state: GateState,
    button_state: ButtonState,
    time: RaceInstant,
}

impl ReadyAppState {
    pub fn update(&mut self, services: &Services) -> AppState {
        let is_wifi_connected = services.platform.wifi().is_connected();
        let gate_state = services.platform.gate().state();
        let button_state = services.platform.button().state();
        let time = services.race_clock.now().expect("Cannot get time");

        AppState::Ready(ReadyAppState {
            is_wifi_connected,
            gate_state,
            button_state,
            time,
        })
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct RaceInstant(u16);

impl RaceInstant {
    pub fn from_millis(ms: u16) -> Self {
        Self(ms)
    }
}

struct RaceClock {
    start: Instant,
}

impl RaceClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    fn now(&self) -> Option<RaceInstant> {
        let t = Instant::now().checked_duration_since(self.start)?;
        let t_ms = t.as_millis();

        // milliseconds, 16 bits, max 18 hours
        if t_ms < (u16::MAX as u128) {
            Some(RaceInstant(t_ms as u16))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
