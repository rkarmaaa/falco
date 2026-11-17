# 0005 · Tray e runtime del core

- **Stato**: accettato
- **Data**: 2026-11-17
- **Sezioni della specifica**: §2, §10, §13

## Decisione

- Thread principale: loop `tao` con `ControlFlow::Wait`, quindi nessun risveglio a riposo. Il tray si crea su `StartCause::Init`, requisito di macOS.
- I clic del menu arrivano al loop con `MenuEvent::set_event_handler` e un `EventLoopProxy`; il loop li inoltra al runtime con un canale `mpsc`.
- Il runtime aggiorna il tray (stato, «Disponibile», lingua) inviando `TrayUpdate` al loop.
- Nessun polling in M0: lo stato di Tailscale si legge su richiesta (`state.snapshot`, `net.runDoctor`), con timeout di 5 s e senza finestra di console su Windows.
- Log con `tracing-appender`: rotazione giornaliera, 7 file, livello da `advanced.log_level`, sovrascrivibile con la variabile `FALO_LOG`.
- In release su Windows il core è un'app senza console (`windows_subsystem = "windows"`).
