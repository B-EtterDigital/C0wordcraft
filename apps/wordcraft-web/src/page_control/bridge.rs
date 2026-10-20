//! Browser wiring adapted from PhotoCraft's page bridge, without image/session-specific requests.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use serde_json::{Value, json};
use wasm_bindgen::{JsCast as _, prelude::*};
use wordcraft_ui_egui::{ControlRequest, WordApp};

const PAGE_FUNCTION: &str = "__wordcraftControl";
const PUMP_EVENT: &str = "wordcraft-control-pass";

thread_local! {
    static RUNNER: RefCell<Option<eframe::WebRunner>> = const { RefCell::new(None) };
    static PUMP: RefCell<Option<Pump>> = const { RefCell::new(None) };
}

pub fn runner(runner: &eframe::WebRunner) {
    RUNNER.with(|slot| *slot.borrow_mut() = Some(runner.clone()));
}

#[derive(Clone)]
struct Pump {
    target: web_sys::EventTarget,
    ran: Rc<Cell<bool>>,
    last: Rc<Cell<f64>>,
}

impl Pump {
    fn get() -> Option<Self> {
        if let Some(pump) = PUMP.with(|slot| slot.borrow().clone()) {
            return Some(pump);
        }
        let runner = RUNNER.with(|slot| slot.borrow().clone())?;
        let pump = Self { target: web_sys::EventTarget::new().ok()?, ran: Rc::default(), last: Rc::default() };
        let ran = pump.ran.clone();
        runner
            .add_event_listener::<web_sys::Event>(&pump.target, PUMP_EVENT, move |_event, app_runner| {
                app_runner.logic();
                ran.set(true);
            })
            .ok()?;
        PUMP.with(|slot| *slot.borrow_mut() = Some(pump.clone()));
        Some(pump)
    }

    fn pass(&self) -> bool {
        self.ran.set(false);
        let Ok(event) = web_sys::Event::new(PUMP_EVENT) else { return false };
        let _ = self.target.dispatch_event(&event);
        if self.ran.get() {
            self.last.set(js_sys::Date::now());
        }
        self.ran.get()
    }

    fn keep_going(&self, ctx: &egui::Context) {
        if super::pump_due(ctx.has_requested_repaint(), js_sys::Date::now(), self.last.get()) {
            self.pass();
        }
    }
}

pub fn attach(mut app: WordApp, ctx: &egui::Context) -> WordApp {
    let (tx, rx) = channel();
    super::files::attach(&mut app, ctx);
    install(tx, ctx.clone());
    app.with_control(rx)
}

fn install(tx: Sender<ControlRequest>, ctx: egui::Context) {
    let Some(window) = web_sys::window() else {
        log::error!("page-control: no window");
        return;
    };
    let call = Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |text: JsValue| {
        let request = text.as_string().ok_or_else(|| "request must be JSON text".to_owned()).and_then(|text| super::parse(&text));
        let (method, params) = match request {
            Ok(request) => request,
            Err(error) => return resolved_error(error),
        };
        let reads_only = super::reads_only(&method, &params);
        let inline = super::inline_passes(&method);
        let (request, rx) = ControlRequest::new(method, params);
        if tx.send(request).is_err() {
            return resolved_error("WordCraft has stopped");
        }
        let pump = Pump::get();
        if !pump.as_ref().is_some_and(Pump::pass) {
            ctx.request_repaint();
        } else {
            for _ in 0..inline {
                if !pump.as_ref().is_some_and(Pump::pass) {
                    break;
                }
            }
            if !reads_only {
                ctx.request_repaint_after(std::time::Duration::from_millis(1));
            }
        }
        wasm_bindgen_futures::future_to_promise(wait(rx, ctx.clone(), pump))
    });
    if let Err(error) = js_sys::Reflect::set(&window, &JsValue::from_str(PAGE_FUNCTION), call.as_ref().unchecked_ref()) {
        log::error!("page-control: cannot install {PAGE_FUNCTION}: {error:?}");
        return;
    }
    call.forget();
}

fn resolved_error(error: impl std::fmt::Display) -> js_sys::Promise {
    js_sys::Promise::resolve(&JsValue::from_str(&encode(&json!({"ok": false, "error": error.to_string()}))))
}

async fn wait(rx: Receiver<Value>, ctx: egui::Context, pump: Option<Pump>) -> Result<JsValue, JsValue> {
    loop {
        match rx.try_recv() {
            Ok(value) => return Ok(JsValue::from_str(&encode(&value))),
            Err(TryRecvError::Disconnected) => {
                return Ok(JsValue::from_str(&encode(&json!({"ok": false, "error": "WordCraft dropped the request"}))));
            }
            Err(TryRecvError::Empty) => {
                sleep().await;
                if let Some(pump) = &pump {
                    pump.keep_going(&ctx);
                }
            }
        }
    }
}

async fn sleep() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window()
            && window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, super::POLL_MS).is_ok()
        {
            return;
        }
        let _ = resolve.call0(&JsValue::NULL);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

fn encode(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| r#"{"ok":false,"error":"reply could not be encoded"}"#.into())
}
