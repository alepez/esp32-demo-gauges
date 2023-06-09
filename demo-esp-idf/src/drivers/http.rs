use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;
use std::{thread::sleep, time::Duration};

use embedded_svc::http::Method;
use embedded_svc::io::Write;
use embedded_svc::ws::FrameType;
use esp_idf_svc::http::server::ws::EspHttpWsDetachedSender;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_sys::EspError;
use racegate::app::SystemState;

struct StateSender {
    ws: EspHttpWsDetachedSender,
}

#[derive(Clone)]
struct StateSenders(Arc<Mutex<VecDeque<StateSender>>>);

impl StateSenders {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::<StateSender>::new())))
    }

    fn add(&self, ws: EspHttpWsDetachedSender) {
        log::info!("detached sender created");
        if let Ok(mut senders) = self.0.lock() {
            log::info!("detached sender added");
            senders.push_back(StateSender { ws });
            log::info!("detached senders count: {}", senders.len());
        }
    }

    fn send(&self, system_state: &SystemState) {
        let frame_type = FrameType::Binary(false);

        let json = serde_json::to_vec(&system_state).unwrap();
        let data = json.as_slice();

        // try_lock is used because we want to avoid waiting for the lock to be
        // acquired and we accept to miss some transmission.
        if let Ok(mut senders) = self.0.try_lock() {
            let mut err_count = 0;

            // FIXME Clean up closed
            let senders: &mut VecDeque<StateSender> = &mut senders;

            for sender in senders.iter_mut().filter(|x| !x.ws.is_closed()) {
                if sender.ws.send(frame_type, data).is_err() {
                    err_count += 1;
                }
            }

            if err_count > 0 {
                log::error!("error sending gate status");
            }
        }
    }

    fn cleanup_closed(&self) {
        if let Ok(mut senders) = self.0.try_lock() {
            let pre_count = senders.len();
            senders.retain(|x| !x.ws.is_closed());
            let removed_count = pre_count - senders.len();
            if removed_count > 0 {
                log::info!("removed {}", removed_count);
            }
        }
    }
}

pub struct HttpServer {
    #[allow(dead_code)]
    esp_http_server: EspHttpServer,
    app_state: Arc<Mutex<SystemState>>,
    #[allow(dead_code)]
    send_task: JoinHandle<()>,
}

fn add_handlers(server: &mut EspHttpServer) -> anyhow::Result<StateSenders> {
    let state_senders = StateSenders::new();
    let state_senders_copy = state_senders.clone();

    server.fn_handler("/", Method::Get, |request| {
        let mut response = request.into_ok_response()?;
        response.write_all(index_html())?;
        Ok(())
    })?;

    server.fn_handler("/style.css", Method::Get, |request| {
        let headers = [("Content-Type", "text/css")];
        let mut response = request.into_response(200, None, &headers)?;
        response.write_all(ui_css())?;
        Ok(())
    })?;

    server.fn_handler("/racegate-ui.js", Method::Get, |request| {
        let headers = [("Content-Type", "application/javascript")];
        let mut response = request.into_response(200, None, &headers)?;
        response.write_all(ui_js())?;
        Ok(())
    })?;

    server.fn_handler("/racegate-ui_bg.wasm", Method::Get, |request| {
        let headers = [
            ("Content-Type", "application/wasm"),
            ("Content-Encoding", "gzip"),
        ];
        let mut response = request.into_response(200, None, &headers)?;
        response.write_all(ui_wasm())?;
        Ok(())
    })?;

    // TODO this file name may change
    server.fn_handler(
        "/snippets/dioxus-interpreter-js-1676574062e4c953/inline0.js",
        Method::Get,
        |request| {
            let headers = [("Content-Type", "application/javascript")];
            let mut response = request.into_response(200, None, &headers)?;
            response.write_all(dioxus_interpreter())?;
            Ok(())
        },
    )?;

    server.ws_handler("/test", |conn| -> Result<(), EspError> {
        let frame_type = FrameType::Binary(false);
        let data = "test".as_bytes();
        loop {
            if let Err(err) = conn.send(frame_type, data) {
                log::info!("error: {}", err);
                break;
            }
            sleep(Duration::from_millis(10));
        }
        Ok(())
    })?;

    server.ws_handler("/state", move |conn| -> Result<(), EspError> {
        if conn.is_new() {
            if let Ok(detached_sender) = conn.create_detached_sender() {
                state_senders_copy.add(detached_sender);
            }
        }
        Ok(())
    })?;

    Ok(state_senders)
}

fn spawn_send_task(state_senders: StateSenders, state: Arc<Mutex<SystemState>>) -> JoinHandle<()> {
    const TASK_WAKEUP_PERIOD: Duration = Duration::from_millis(250);

    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || loop {
            let start = Instant::now();

            let next_wakeup = Instant::now() + TASK_WAKEUP_PERIOD;

            state_senders.cleanup_closed();

            // Instead of keeping the mutex locked until the state is sent, we get
            // a copy of the state and send it asynchronously.
            if let Ok(state) = state.try_lock().map(|x| x.clone()) {
                state_senders.send(&state);
            }

            log::trace!("ws update took {}ms", (Instant::now() - start).as_millis());

            // Ensure this task is not spinning
            if let Some(delay) = next_wakeup.checked_duration_since(Instant::now()) {
                sleep(delay);
            } else {
                log::error!("no delay");
            }
        })
        .unwrap()
}

impl HttpServer {
    pub fn new() -> anyhow::Result<Self> {
        let conf = Configuration::default();
        let mut esp_http_server = EspHttpServer::new(&conf)?;
        let app_state = Arc::new(Mutex::new(Default::default()));
        let state_senders = add_handlers(&mut esp_http_server)?;

        let send_task = spawn_send_task(state_senders.clone(), app_state.clone());

        Ok(HttpServer {
            esp_http_server,
            app_state,
            send_task,
        })
    }
}

impl racegate::svc::HttpServer for HttpServer {
    fn set_system_state(&self, state: &SystemState) {
        // try_lock is used because we want to avoid waiting for the lock to be
        // acquired and we accept to miss some update.
        self.app_state
            .try_lock()
            .as_mut()
            .map(|x| {
                **x = state.clone();
            })
            .ok();
    }
}

fn index_html() -> &'static [u8] {
    include_bytes!("../../../racegate-ui/dist/index.html")
}

fn ui_js() -> &'static [u8] {
    include_bytes!("../../../racegate-ui/dist/racegate-ui.js")
}

fn ui_wasm() -> &'static [u8] {
    include_bytes!("../../../racegate-ui/dist/racegate-ui_bg.wasm.gz")
}

fn dioxus_interpreter() -> &'static [u8] {
    // TODO this file name may change
    include_bytes!(
        "../../../racegate-ui/dist/snippets/dioxus-interpreter-js-1676574062e4c953/inline0.js"
    )
}

fn ui_css() -> &'static [u8] {
    include_bytes!("../../../racegate-ui/dist/style.css")
}
