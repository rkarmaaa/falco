//! Volume dello stream per chi guarda: agisce solo sulla sessione audio di Moonlight
//! (come il mixer di Windows), non sul volume del sistema né sulle altre app.
#![allow(unsafe_code)]

/// Imposta volume (0–1) e muto dello stream. `false` se l'audio di Moonlight non è ancora partito.
#[tauri::command]
pub async fn stream_volume(pid: u32, volume: f32, muted: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(move || win::set(pid, volume.clamp(0.0, 1.0), muted))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, volume, muted);
        Ok(false)
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::Media::Audio::{
        IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume,
        MMDeviceEnumerator, eConsole, eRender,
    };
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::core::Interface;

    pub fn set(pid: u32, volume: f32, muted: bool) -> windows::core::Result<bool> {
        // SAFETY: COM inizializzato su questo thread (di lavoro); interfacce usate solo qui
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let sessions = manager.GetSessionEnumerator()?;
            let mut found = false;
            for i in 0..sessions.GetCount()? {
                let session = sessions.GetSession(i)?;
                let control: IAudioSessionControl2 = session.cast()?;
                if control.GetProcessId().unwrap_or(0) != pid {
                    continue;
                }
                let simple: ISimpleAudioVolume = session.cast()?;
                simple.SetMasterVolume(volume, std::ptr::null())?;
                simple.SetMute(muted, std::ptr::null())?;
                found = true;
            }
            Ok(found)
        }
    }
}
