use std::{rc::Rc, time::Duration};

use dioxus::prelude::*;
use dioxus_websocket_hooks::use_ws_context_provider_json;
use fermi::{use_init_atom_root, use_read, use_set, Atom};
use racegate::app::{gates::Gate, SystemState};
use racegate::CoordinatedInstant;

pub static SYSTEM_STATE: Atom<Option<SystemState>> = |_| None;

#[allow(non_snake_case)]
pub fn App(cx: Scope) -> Element {
    use_init_atom_root(cx);
    let set_system_state = Rc::clone(use_set(cx, SYSTEM_STATE));

    let ws_url = ws_url_from_hostname();

    use_ws_context_provider_json::<SystemState>(cx, &ws_url, move |msg| {
        set_system_state(Some(msg));
    });

    cx.render(rsx!(
        h1 { "racegate" },
        Main { },
    ))
}

#[allow(non_snake_case)]
fn Main(cx: Scope) -> Element {
    if let Some(system_state) = use_read(cx, SYSTEM_STATE) {
        cx.render(rsx!(Dashboard {
            system_state: system_state.clone()
        }))
    } else {
        cx.render(rsx!(div { "loading..." }))
    }
}

#[allow(non_snake_case)]
#[inline_props]
pub fn Dashboard(cx: Scope<'a>, system_state: SystemState) -> Element {
    let duration = system_state.race.duration();

    let start_gate = system_state.gates.start_gate().clone();
    let finish_gate = system_state.gates.finish_gate().clone();

    cx.render(rsx!(
        DurationComponent { duration: duration },
        GateComponent {
            name: "Start".to_owned(),
            gate: start_gate,
            time: system_state.time
        },
        GateComponent {
            name: "Finish".to_owned(),
            gate: finish_gate,
            time: system_state.time
        },
    ))
}

#[allow(non_snake_case)]
#[inline_props]
fn DurationComponent(cx: Scope, #[props(!optional)] duration: Option<Duration>) -> Element {
    let duration_text = duration
        .map(format_duration)
        .unwrap_or_else(|| "-".to_owned());

    cx.render(rsx!(
        div {
            class: "duration",
            span { duration_text }
        }
    ))
}

fn format_duration(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64())
}

fn time_since_gate_activation(gate: &Gate, time: &CoordinatedInstant) -> String {
    let Some(t) = gate.last_activation_time else {
        return "-".to_owned();
    };

    let diff = Duration::from_millis((time.as_millis() - t.as_millis()) as u64);

    format_duration(diff)
}

#[allow(non_snake_case)]
#[inline_props]
fn GateComponent(cx: Scope, name: String, gate: Gate, time: CoordinatedInstant) -> Element {
    let alive = gate.is_alive(*time);
    let active = gate.is_active();

    let alive_class = if alive { "gate-alive" } else { "gate-dead" };

    let active_class = if active {
        "gate-active"
    } else {
        "gate-inactive"
    };

    let time = time_since_gate_activation(&gate, &time);

    cx.render(rsx!(
        div {
            class: "gate",
            span {
                class: "gate-name",
                "{name}"
            }
            span {
                class: "gate-time",
                "{time}",
            }
            span {
                class: alive_class,
                "{alive}",
            }
            span {
                class: active_class,
                "{active}",
            }
        }
    ))
}

fn hostname() -> Option<String> {
    #[cfg(target_family = "wasm")]
    {
        let window = web_sys::window()?;
        Some(window.location().hostname().ok()?.to_string())
    }
    #[cfg(not(target_family = "wasm"))]
    {
        None
    }
}

fn ws_url_from_hostname() -> String {
    const DEFAULT_HOSTNAME: &'static str = "192.168.1.71.1";
    let h = hostname().unwrap_or_else(|| DEFAULT_HOSTNAME.to_owned());
    format!("ws://{h}/state")
}
