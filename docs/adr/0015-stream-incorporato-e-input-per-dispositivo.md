# ADR 0015 — Stream dentro la finestra e input per dispositivo

- Stato: accettato
- Data: 2026-11-18

## Contesto

Richieste del redesign: collegandosi a qualcuno l'app deve restare a dimensione normale, con le azioni
nella barra in basso e «Schermo intero» in fondo a destra; a schermo intero serve un pulsantino discreto
per uscire. L'host deve vedere chi è collegato, con tastiera, mouse e controller che si illuminano
quando ricevono input; di default ogni ospite può solo guardare e col tasto destro si concedono i
singoli dispositivi.

Vincoli: Moonlight disegna il video in una finestra sua (SDL); Sunshine 2026.914 non ha permessi di
input per client, solo interruttori globali `keyboard`, `mouse`, `controller`.

## Decisione

### Stream incorporato (Windows)

- Moonlight parte sempre `--display-mode windowed`. Il core emette `session.started { pid, … }` e non
  chiude più la UI.
- `falco-ui` (`src-tauri/src/embed.rs`) cerca la finestra `SDL_app` del processo, ne toglie i bordi e la
  rende **figlia** della finestra di Falco (`SetParent`, `WS_CHILD`), poi la posiziona nell'area del
  palco che la pagina misura (`stream_place`, a ogni ridimensionamento). La schermata Qt di avvio di
  Moonlight viene nascosta.
- Una finestra figlia sta sempre sopra la webview: ogni pannello che si apre sopra il palco (modali,
  menu) la nasconde finché è aperto (`useStageCover`); gli avvisi si affiancano al palco invece di
  coprirlo.
- «Schermo intero» porta a pieno schermo la finestra di Falco: il palco occupa tutto e il flusso lo
  segue. Il pulsantino è una seconda finestra Tauri piccola, trasparente e sempre in primo piano
  (`index.html#pill`): quasi invisibile finché non ci si passa sopra, con «Esci da schermo intero» ed
  «Esci» (due clic). Il comando che la crea è asincrono: su Windows creare finestre da un comando
  sincrono blocca Tauri.
- Chiudere la finestra durante uno stream esce prima dalla sessione (`session.leave`).
- macOS: nessun incorporamento; Moonlight resta nella sua finestra e la UI lo dice.

È l'unico codice `unsafe` della UI (poche chiamate Win32, ognuna con il suo commento `SAFETY`).
Verificato in loopback su Windows 11 a 3840×2160 con scala 150%: incorporamento, ridimensionamento,
schermo intero, pulsantino, uscita.

### Input per dispositivo

- I permessi restano per amico (ruolo + override `keyboard`, `mouse`, `gamepads`). Il ruolo di default
  degli inviti è `spettatore`.
- In Sunshine si accendono solo i dispositivi che **almeno un amico** può usare (unione). Senza nessun
  permesso concesso, tastiera, mouse e controller sono spenti e gli ospiti possono solo guardare.
- Cambiare l'unione richiede il riavvio di Sunshine: senza ospiti avviene subito; con ospiti collegati si
  aspetta la fine della sessione oppure l'host sceglie «Applica ora» (`host.applyInputs`), che riavvia e
  fa ricollegare gli ospiti.
- Limite noto: con due ospiti, se uno ha la tastiera anche l'altro tecnicamente la può usare. La
  soluzione completa (permessi per client dentro Sunshine) richiede il fork previsto dal §8.1.

### Attività degli input

- `apps/core/src/input_monitor.rs`, attivo solo con ospiti collegati e almeno un dispositivo acceso:
  hook di basso livello (`willhook`) che contano solo gli eventi **iniettati** (quelli di Sunshine;
  l'input fisico dell'host è escluso) e stato XInput (`rusty-xinput`) dei controller comparsi dopo
  l'avvio del monitor (quelli virtuali degli ospiti).
- Evento `host.input { keyboard, mouse, controller }` al massimo 4 volte al secondo; la UI illumina le
  icone degli ospiti che hanno quel dispositivo.

## Conseguenze

- L'esperienza è quella di Parsec: lo stream vive dentro l'app, le azioni sono sempre a portata.
- Due dipendenze leggere solo su Windows (`willhook`, `rusty-xinput`) e `windows-sys`, già presente tra
  quelle di Tauri.
- Gli hook di basso livello aggiungono un passaggio a ogni evento di input dell'host: per questo girano
  solo mentre qualcuno è collegato.
