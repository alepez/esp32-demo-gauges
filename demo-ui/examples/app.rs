#![allow(non_snake_case)]

use std::time::Duration;

use dioxus_desktop::Config as DesktopConfig;
use racegate::app::{Gate, Gates, Race, SystemState};
use racegate::svc::CoordinatedInstant;
use racegate_ui::app::{Dashboard, DashboardProps};

fn test_default() -> SystemState {
    SystemState::default()
}

fn test_start_gate_active() -> SystemState {
    SystemState {
        time: CoordinatedInstant::from_millis(1000),
        gates: Gates::new([
            Gate {
                active: true,
                last_activation_time: Some(CoordinatedInstant::from_millis(1000)),
                last_beacon_time: Some(CoordinatedInstant::from_millis(1000)),
            },
            Gate::default(),
            Gate::default(),
            Gate::default(),
        ]),
        race: Race {
            start_time: None,
            finish_time: None,
            duration: None,
        },
    }
}

fn test_race_finished() -> SystemState {
    SystemState {
        time: CoordinatedInstant::from_millis(5000),
        gates: Gates::new([
            Gate {
                active: false,
                last_activation_time: Some(CoordinatedInstant::from_millis(1000)),
                last_beacon_time: Some(CoordinatedInstant::from_millis(5000)),
            },
            Gate::default(),
            Gate::default(),
            Gate {
                active: true,
                last_activation_time: Some(CoordinatedInstant::from_millis(3456)),
                last_beacon_time: Some(CoordinatedInstant::from_millis(5000)),
            },
        ]),
        race: Race {
            start_time: None,
            finish_time: None,
            duration: Some(Duration::from_millis(2456)),
        },
    }
}

fn test_gate_dead() -> SystemState {
    SystemState {
        time: CoordinatedInstant::from_millis(5000),
        gates: Gates::new([
            Gate {
                active: false,
                last_activation_time: Some(CoordinatedInstant::from_millis(1000)),
                last_beacon_time: Some(CoordinatedInstant::from_millis(0)),
            },
            Gate::default(),
            Gate::default(),
            Gate {
                active: true,
                last_activation_time: Some(CoordinatedInstant::from_millis(3456)),
                last_beacon_time: Some(CoordinatedInstant::from_millis(0)),
            },
        ]),
        race: Race {
            start_time: None,
            finish_time: None,
            duration: Some(Duration::from_millis(2456)),
        },
    }
}

fn custom_head() -> String {
    r#"<link rel="stylesheet" href="assets/style.css" />"#.to_owned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let table: Vec<fn() -> SystemState> = vec![
        test_default,
        test_start_gate_active,
        test_race_finished,
        test_gate_dead,
    ];

    let args: Vec<_> = std::env::args().collect();
    let test_id = args.get(1).map(|x| x.parse().unwrap()).unwrap_or(0);
    let test = table.get(test_id).expect("Invalid test id");
    let system_state = test();

    let config = DesktopConfig::new().with_custom_head(custom_head());

    let props = DashboardProps { system_state };
    dioxus_desktop::launch_with_props(Dashboard, props, config);
    Ok(())
}
