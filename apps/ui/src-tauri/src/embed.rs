//! Stream dentro la finestra di Falco (ADR 0015): la finestra di Moonlight diventa figlia
//! della nostra e occupa l'area che la UI le riserva. Così l'app resta a dimensione normale
//! con la barra delle azioni sotto, e «Schermo intero» porta a pieno schermo tutta la finestra.
//!
//! Unico punto con codice `unsafe` della UI: poche chiamate Win32 documentate.
#![allow(unsafe_code)]
// Fuori da Windows il flusso resta nella finestra di Moonlight: parte del codice non serve
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::{State, WebviewWindow};

/// Area del flusso in pixel CSS, relativa alla finestra.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Finestra di Moonlight incorporata (HWND come intero: i puntatori non sono `Send`).
#[derive(Default)]
pub struct Embedded(Mutex<Option<isize>>);

fn physical(win: &WebviewWindow, r: Rect) -> (i32, i32, i32, i32) {
    let s = win.scale_factor().unwrap_or(1.0);
    let p = |v: f64| (v * s).round() as i32;
    (p(r.x), p(r.y), p(r.w).max(1), p(r.h).max(1))
}

#[cfg(windows)]
mod win32 {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetWindowLongPtrW,
        GetWindowThreadProcessId, HWND_TOP, IsWindow, IsWindowVisible, SW_HIDE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_SHOWWINDOW, SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        WS_CAPTION, WS_CHILD, WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_EX_DLGMODALFRAME,
        WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    };

    struct Search {
        pid: u32,
        stream: Option<isize>,
        others: Vec<isize>,
        classes: Vec<String>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> i32 {
        // SAFETY: lparam è il puntatore a `Search` passato da `find`, valido per tutta EnumWindows
        let s = unsafe { &mut *(lparam as *mut Search) };
        let mut pid = 0u32;
        // SAFETY: hwnd arriva da EnumWindows; pid è una variabile locale
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        // SAFETY: hwnd valido durante l'enumerazione
        if pid != s.pid || unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut buf = [0u16; 64];
        // SAFETY: buffer locale di 64 caratteri, la lunghezza passata coincide
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        // Le finestre Qt sono la schermata di avvio; il flusso è l'altra (SDL, anche via
        // sdl2-compat su SDL3, che può registrare una classe con un nome diverso da «SDL_app»)
        if class.starts_with("Qt") {
            s.others.push(hwnd as isize);
        } else if s.stream.is_none() || class.starts_with("SDL") {
            s.stream = Some(hwnd as isize);
        }
        s.classes.push(class);
        1
    }

    /// Finestra del flusso del processo `pid`, le sue altre finestre visibili e le classi viste.
    pub fn find(pid: u32) -> (Option<isize>, Vec<isize>, Vec<String>) {
        let mut s = Search { pid, stream: None, others: Vec::new(), classes: Vec::new() };
        // SAFETY: `visit` usa `s` solo durante la chiamata sincrona
        unsafe { EnumWindows(Some(visit), &mut s as *mut Search as LPARAM) };
        (s.stream, s.others, s.classes)
    }

    pub fn hide(hwnd: isize) {
        // SAFETY: ShowWindow su un HWND non valido fallisce senza effetti
        unsafe { ShowWindow(hwnd as HWND, SW_HIDE) };
    }

    pub fn alive(hwnd: isize) -> bool {
        // SAFETY: IsWindow accetta qualsiasi valore
        unsafe { IsWindow(hwnd as HWND) != 0 }
    }

    /// Rende `child` figlia di `parent`, senza bordi, nell'area indicata.
    pub fn attach(parent: isize, child: isize, (x, y, w, h): (i32, i32, i32, i32)) {
        let c = child as HWND;
        // SAFETY: stili letti e scritti sulla stessa finestra; SetParent tra finestre valide
        unsafe {
            let style = GetWindowLongPtrW(c, GWL_STYLE) as u32;
            let style = (style
                & !(WS_POPUP
                    | WS_CAPTION
                    | WS_THICKFRAME
                    | WS_SYSMENU
                    | WS_MINIMIZEBOX
                    | WS_MAXIMIZEBOX))
                | WS_CHILD
                | WS_CLIPSIBLINGS;
            SetWindowLongPtrW(c, GWL_STYLE, style as isize);
            let ex = GetWindowLongPtrW(c, GWL_EXSTYLE) as u32;
            let ex = ex & !(WS_EX_APPWINDOW | WS_EX_DLGMODALFRAME | WS_EX_WINDOWEDGE);
            SetWindowLongPtrW(c, GWL_EXSTYLE, ex as isize);
            SetParent(c, parent as HWND);
            SetWindowPos(c, HWND_TOP, x, y, w, h, SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        }
    }

    pub fn place(child: isize, (x, y, w, h): (i32, i32, i32, i32)) {
        // SAFETY: SetWindowPos su un HWND non valido fallisce senza effetti
        unsafe {
            SetWindowPos(child as HWND, HWND_TOP, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW)
        };
    }

    pub fn conceal(child: isize) {
        hide(child);
    }

    /// Scorciatoia di Moonlight sul flusso (es. Ctrl+Alt+Maiusc+S per le statistiche): focus
    /// alla finestra, poi tasti premuti e rilasciati in ordine inverso.
    pub fn shortcut(child: isize, key: u16) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_CONTROL,
            VK_MENU, VK_SHIFT,
        };
        focus(child);
        let ev = |vk: u16, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let keys = [VK_CONTROL, VK_MENU, VK_SHIFT, key];
        let mut seq: Vec<INPUT> = keys.iter().map(|&k| ev(k, false)).collect();
        seq.extend(keys.iter().rev().map(|&k| ev(k, true)));
        // SAFETY: array di INPUT valido per tutta la chiamata, dimensione della struttura corretta
        unsafe { SendInput(seq.len() as u32, seq.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    /// Focus della tastiera al flusso: senza, i tasti restano alla pagina di Falco e non
    /// arrivano a Moonlight (clic e movimenti sì, perché vanno alla finestra sotto il cursore).
    pub fn focus(child: isize) {
        use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
        let c = child as HWND;
        // SAFETY: thread della finestra figlia collegato al nostro solo per la durata di SetFocus
        unsafe {
            let target = GetWindowThreadProcessId(c, std::ptr::null_mut());
            let me = GetCurrentThreadId();
            let attached = target != me && AttachThreadInput(me, target, 1) != 0;
            SetFocus(c);
            if attached {
                AttachThreadInput(me, target, 0);
            }
        }
    }
}

/// Aspetta la finestra del flusso di `pid` e la incorpora. `false` se non compare.
#[tauri::command]
pub async fn stream_attach(
    window: WebviewWindow,
    state: State<'_, Embedded>,
    pid: u32,
    rect: Rect,
) -> Result<bool, String> {
    #[cfg(windows)]
    {
        let parent = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        // Moonlight apre la finestra solo a connessione avvenuta: fino a 90 s
        let mut seen = Vec::new();
        for _ in 0..900 {
            let (stream, others, classes) = win32::find(pid);
            // Le finestre Qt di Moonlight (avvio, errori) non devono comparire sopra Falco
            others.into_iter().for_each(win32::hide);
            if let Some(child) = stream {
                win32::attach(parent, child, physical(&window, rect));
                *state.0.lock().map_err(|e| e.to_string())? = Some(child);
                return Ok(true);
            }
            seen = classes;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(format!("finestra del flusso non trovata (finestre di Moonlight: {seen:?})"))
    }
    #[cfg(not(windows))]
    {
        let _ = (window, state, pid, rect);
        Ok(false)
    }
}

/// Sposta il flusso nella nuova area (ridimensionamento, schermo intero).
#[tauri::command]
pub fn stream_place(window: WebviewWindow, state: State<'_, Embedded>, rect: Rect) -> bool {
    #[cfg(windows)]
    {
        let Ok(mut guard) = state.0.lock() else { return false };
        match *guard {
            Some(child) if win32::alive(child) => {
                win32::place(child, physical(&window, rect));
                true
            }
            _ => {
                *guard = None;
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, state, rect);
        false
    }
}

/// Dà la tastiera al flusso incorporato. Sincrono: gira sul thread della finestra di Falco.
#[tauri::command]
pub fn stream_focus(state: State<'_, Embedded>) -> bool {
    #[cfg(windows)]
    {
        let Ok(guard) = state.0.lock() else { return false };
        match *guard {
            Some(child) if win32::alive(child) => {
                win32::focus(child);
                true
            }
            _ => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        false
    }
}

/// Statistiche di Moonlight sopra il video (Ctrl+Alt+Maiusc+S): l'unico modo di vederle dal vivo.
#[tauri::command]
pub fn stream_stats(state: State<'_, Embedded>) -> bool {
    #[cfg(windows)]
    {
        let Ok(guard) = state.0.lock() else { return false };
        match *guard {
            Some(child) if win32::alive(child) => {
                win32::shortcut(child, u16::from(b'S'));
                true
            }
            _ => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        false
    }
}

/// Nasconde il flusso (finestra di dialogo aperta sopra) o lo dimentica a fine sessione.
#[tauri::command]
pub fn stream_detach(state: State<'_, Embedded>, hide: bool) {
    let Ok(mut guard) = state.0.lock() else { return };
    #[cfg(windows)]
    if let Some(child) = *guard
        && hide
    {
        win32::conceal(child);
        return;
    }
    #[cfg(not(windows))]
    let _ = hide;
    *guard = None;
}
