use angle::Rad;
use demo::app::SystemState;
use dioxus::prelude::*;
use dioxus_websocket_hooks::use_ws_context_provider_json;
use fermi::{use_init_atom_root, use_read, use_set, Atom};
use gauges::app::gauge::Gauge;
use gauges::prelude::*;
use std::f64::consts::PI;
use std::rc::Rc;

pub static SYSTEM_STATE: Atom<Option<SystemState>> = |_| None;

#[allow(non_snake_case)]
pub fn App(cx: Scope) -> Element {
    use_init_atom_root(cx);
    let set_system_state = Rc::clone(use_set(cx, SYSTEM_STATE));

    let ws_url = ws_url_from_hostname();

    use_ws_context_provider_json::<SystemState>(cx, &ws_url, move |msg| {
        set_system_state(Some(msg));
    });

    cx.render(rsx!(Main {},))
}

const ARC_STYLE: GaugeStyle = GaugeStyle::Arc(ArcGaugeStyle {
    radius: 50.0,
    begin_angle: Rad(225.0 / 180.0 * PI),
    full_width: Rad(270.0 / 180.0 * PI),
});

#[allow(non_snake_case)]
fn Main(cx: Scope) -> Element {
    if let Some(system_state) = use_read(cx, SYSTEM_STATE) {
        let value = Value::Float(system_state.counter as f64);
        let signal = SignalInfo {
            name: Some("Counter".to_owned()),
        };
        let style = ARC_STYLE;
        let range = Range {
            min: 0.,
            max: 1_000_000.,
        };
        let format = GaugeTextFormat { precision: 0 };
        let age = Age::New;

        cx.render(rsx! {
            Gauge {
                value: value,
                signal: signal,
                style: style,
                range: range,
                format: format,
                age: age,
            }
        })
    } else {
        cx.render(rsx!(div { "loading..." }))
    }
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
    const DEFAULT_HOSTNAME: &'static str = "192.168.71.1";
    let h = hostname().unwrap_or_else(|| DEFAULT_HOSTNAME.to_owned());
    format!("ws://{h}/state")
}
