# 0001 · Monorepo e stack

- **Stato**: accettato
- **Data**: 2026-11-17
- **Sezioni della specifica**: §3, §10

## Contesto

Falco ha un demone sempre attivo, una UI aperta di rado e due motori C++ forkati. Il demone deve stare sotto 15 MB di RAM e non svegliarsi quasi mai.

## Decisione

- Workspace Cargo unico: `apps/core` (binario `falco-core`), `apps/ui/src-tauri` (binario `falco-ui`), `crates/falco-{proto,net,store,engines}`.
- Il core non contiene webview: tray con `tray-icon` sul loop di `tao` nel thread principale (richiesto da macOS), logica su un runtime `tokio` *current-thread* in un secondo thread (ADR 0005).
- UI con Tauri 2 + Vue 3 + TypeScript + Vite + SCSS; nessuna libreria di componenti.
- Profilo di release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.
- **Librerie prima del codice scritto a mano** (richiesta di Marco): `serde_json_canonicalizer`, `toml_edit`, `interprocess`, `which`, `tempfile`, `vue-i18n`, `@vueuse/core`, `lucide-vue-next`, `@fontsource-variable/geist`. Sempre entro i budget del §10.

## Alternative considerate

- Un solo processo Tauri con tray: la webview resterebbe in memoria (decine di MB) anche a finestra chiusa.
- `winit` invece di `tao`: equivalente; `tao` è già nell'ecosistema Tauri.
- Pinia e vue-router: inutili con quattro sezioni e uno stato piccolo; bastano `reactive` e un `ref`.

## Conseguenze

- Due binari da distribuire e firmare. Il core avvia `falco-ui` dalla propria cartella.
- Peso di `tray-icon`, `tao` e `image` nel core da misurare con `cargo bloat` in M1.
