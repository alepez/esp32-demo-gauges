use crate::hal::button::ButtonState;
use crate::hal::rgb_led::RgbLed;
use crate::hal::rgb_led::RgbLedColor;
use crate::hal::Platform;

#[derive(Debug, Default, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemState {}

struct Services<'a> {
    led_controller: LedController<'a>,
    platform: &'a dyn Platform,
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum AppState {
    Init(InitState),
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

        let services = Services {
            led_controller,
            platform,
        };

        let state = AppState::default();

        Self { services, state }
    }

    pub fn update(&mut self) {
        let new_state = match &mut self.state {
            AppState::Init(state) => state.update(&self.services),
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
    }
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
struct InitState {
    button_state: ButtonState,
}

impl InitState {
    pub fn update(&mut self, services: &Services) -> AppState {
        AppState::Init(*self)
    }
}
