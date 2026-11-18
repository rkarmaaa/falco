//! Attività degli ospiti per le icone che si illuminano (tastiera, mouse, controller).
//!
//! Sunshine immette tastiera e mouse con `SendInput`: gli hook di basso livello li vedono
//! marcati come «iniettati», l'input fisico dell'host no. I controller degli ospiti sono
//! virtuali (ViGEm): contano solo quelli comparsi dopo l'avvio del monitor.
//! Il monitor gira solo mentre ci sono ospiti collegati (ADR 0015).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use falco_proto::ipc::{Notification, events};
use serde_json::json;
use tokio::sync::broadcast;

/// Quanto spesso al massimo la UI riceve `host.input`.
const EMIT_EVERY: Duration = Duration::from_millis(250);

pub struct InputMonitor {
    stop: Arc<AtomicBool>,
}

impl Drop for InputMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
struct Activity {
    keyboard: bool,
    mouse: bool,
    controller: bool,
}

impl Activity {
    fn any(self) -> bool {
        self.keyboard || self.mouse || self.controller
    }
}

impl InputMonitor {
    pub fn start(tx: broadcast::Sender<Notification>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let spawned =
            std::thread::Builder::new().name("falco-input".into()).spawn(move || run(&flag, &tx));
        if let Err(e) = spawned {
            tracing::warn!("monitor degli input non avviato: {e}");
        }
        Self { stop }
    }
}

#[cfg(windows)]
fn run(stop: &AtomicBool, tx: &broadcast::Sender<Notification>) {
    use willhook::{InputEvent, IsEventInjected};

    let hook = willhook::willhook();
    if hook.is_none() {
        tracing::warn!("hook degli input non disponibile");
    }
    let pads = rusty_xinput::XInputHandle::load_default().ok();
    // Controller già presenti (quelli dell'host): esclusi
    let mut packets: [Option<u32>; 4] = [None; 4];
    let mut excluded = [false; 4];
    if let Some(x) = &pads {
        for (i, ex) in excluded.iter_mut().enumerate() {
            *ex = x.get_state(i as u32).is_ok();
        }
    }
    let mut seen = Activity::default();
    let mut last = Activity::default();
    let mut last_emit = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if let Some(h) = &hook {
            while let Ok(ev) = h.try_recv() {
                match ev {
                    InputEvent::Keyboard(k) if k.is_injected == Some(IsEventInjected::Injected) => {
                        seen.keyboard = true;
                    }
                    InputEvent::Mouse(m) if m.is_injected == Some(IsEventInjected::Injected) => {
                        seen.mouse = true;
                    }
                    _ => {}
                }
            }
        }
        if let Some(x) = &pads {
            for i in 0..4 {
                if excluded[i] {
                    continue;
                }
                let now = x.get_state(i as u32).ok().map(|s| s.raw.dwPacketNumber);
                if let (Some(n), Some(prev)) = (now, packets[i])
                    && n != prev
                {
                    seen.controller = true;
                }
                packets[i] = now;
            }
        }
        if last_emit.elapsed() >= EMIT_EVERY {
            // Si invia l'attività e anche il ritorno a riposo, una volta
            if seen.any() || last.any() {
                let _ = tx.send(Notification::new(
                    events::HOST_INPUT,
                    json!({ "keyboard": seen.keyboard, "mouse": seen.mouse, "controller": seen.controller }),
                ));
            }
            last = seen;
            seen = Activity::default();
            last_emit = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[cfg(not(windows))]
fn run(_stop: &AtomicBool, _tx: &broadcast::Sender<Notification>) {
    // macOS: in arrivo con l'host per macOS (gli input iniettati si leggono con CGEventTap)
}
