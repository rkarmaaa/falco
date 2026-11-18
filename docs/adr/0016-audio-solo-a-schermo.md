# ADR 0016 — Audio condiviso: solo ciò che è a schermo

- Stato: accettato
- Data: 2026-11-18

## Contesto

Chi usa Falco di solito è anche in chiamata su Discord. Sunshine 2026.914 registra l'intera uscita
audio di Windows (loopback del dispositivo), quindi lo stream porta agli ospiti anche Discord: sentono
due volte chi parla e la propria voce tornare indietro, oltre a notifiche e musica dell'host.

Richiesta di Marco: gli ospiti devono sentire solo l'audio dei programmi le cui finestre sono sullo
schermo condiviso.

Sunshine non ha un'opzione per farlo. Le strade valutate:

| Strada | Problema |
| --- | --- |
| Dispositivo virtuale (Steam Streaming Speakers, VB-Cable) e Discord su un'altra uscita | Serve un driver da installare; con `audio_sink` Sunshine rende il dispositivo virtuale quello predefinito e l'host non sente più nulla; con una sola uscita non funziona |
| Cattura per processo nel core di Falco | Sunshine accetta solo un dispositivo da registrare: senza un driver virtuale l'audio non arriva allo stream |
| **Cattura per processo dentro Sunshine** | Richiede il fork, già previsto dal §3 |

## Decisione

### Patch a Sunshine (`engines/patches/sunshine/0001-*.patch`)

- Nuova opzione `audio_capture = onscreen` (vuota = comportamento upstream) e `audio_exclude_apps`
  (eseguibili separati da virgola, sempre esclusi).
- `src/platform/windows/audio_onscreen.cpp`, attiva solo su Windows:
  - **quali finestre:** ogni mezzo secondo un thread elenca le finestre di primo livello del display
    condiviso, dall'alto in basso, e tiene quelle davvero visibili. Una finestra conta se la parte non
    coperta dalle finestre sopra è almeno lo 0,5% dello schermo. Non contano le finestre minimizzate,
    nascoste da DWM (cloaked), quelle che lasciano passare i clic (overlay) e le tool window (barra
    delle applicazioni, popup). Per le app UWP si risale dal frame di ApplicationFrameHost all'app vera.
  - **quali processi:** per ogni finestra si registra l'albero del processo proprietario con la
    cattura per processo di Windows (`ActivateAudioInterfaceAsync`,
    `PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE`, Windows 10 2004 o successivo). Così si
    prendono anche i processi figli che emettono l'audio, come il servizio audio di Chrome.
  - **mai catturati:** Discord (tutte le varianti), TeamSpeak, Teams, Zoom, Mumble, Skype, Slack,
    WhatsApp, Telegram, Signal, Guilded, Element, Revolt, Vesktop, più `audio_exclude_apps`. Non
    vengono catturati neppure i processi della shell (Explorer e simili), il cui albero conterrebbe
    tutte le app avviate dal desktop.
  - **un albero di processi** che contiene un'app esclusa non viene catturato, altrimenti la farebbe
    passare. Un candidato discendente di un altro candidato viene scartato, per non mixare due volte
    lo stesso audio. Un PID padre riutilizzato da un processo più recente non conta come antenato.
  - **mix:** un thread legge i pacchetti di ogni cattura; `sample()` mixa a ritmo di frame con un
    timer ad alta risoluzione. Ogni sorgente ha un margine di 10 ms contro il jitter: oltre 30 ms
    accumulati butta il più vecchio, e se resta a secco riparte senza buchi. Una sorgente sopravvive
    1,5 s dopo che la sua finestra sparisce, così un alt-tab veloce non taglia l'audio.
- Con `onscreen` Sunshine non cambia più il dispositivo audio predefinito: l'host continua a sentire
  tutto dalle sue casse o cuffie.
- Se Windows non supporta la cattura per processo, Sunshine torna a registrare tutto il dispositivo e
  lo scrive nel log.

### In Falco

- `host.audio_capture = "onscreen" | "all"`, default `onscreen`. Il core scrive `audio_capture =
  onscreen` in `sunshine.conf` solo su Windows.
- UI: «Audio condiviso» nella qualità della condivisione, con «Solo ciò che è a schermo» e «Tutto il PC».
- Build: `scripts/build-sunshine.sh` (MSYS2 UCRT64) clona upstream al commit fissato, applica le patch
  e produce `target/sunshine/sunshine.exe`. `stage-engines.ps1` lo mette al posto di quello
  ufficiale; se manca lo dice e il pacchetto avrà l'audio di tutto il PC.
- Le patch vivono nel repository di Falco finché non esiste il repository del fork `falco-sunshine`
  (serve l'account GitHub di Marco).

### Verifiche

- Sunshine con la patch: 630 test passati, compresi 10 nuovi (`AudioOnscreenTest`). Nove coprono la scelta
  dei processi: chat vocali escluse anche se visibili, shell, alberi duplicati, PID riutilizzati. Il
  decimo avvia la cattura vera e controlla il ritmo dei frame: 100 frame da 5 ms in 506 ms.
- Prova manuale (`FALCO_AUDIO_PROBE`) su Windows 11 build 26300: un tono in una finestra visibile viene
  catturato (picco 0,05, pari al volume generato); un suono da un processo senza finestra no (picco 0).
- Due trappole di Windows trovate durante le prove, e gestite: fermare (`Stop`) un client di cattura per
  processo mai avviato, o rilasciarlo da un thread senza COM, manda in crash `AudioSes.dll`.

## Conseguenze

- Gli ospiti non sentono più Discord né le notifiche dell'host, purché le chat vocali non siano
  avviate da un gioco o da un launcher visibile.
- Un gioco che emette audio da un processo fuori dal proprio albero (raro: alcuni launcher anti-cheat)
  non si sente; la soluzione per l'utente è «Tutto il PC».
- Un'app a schermo ma coperta per oltre il 99,5% non si sente: è il significato di «a schermo».
- Si aggiungono circa 10 ms di buffer audio. Il video non cambia, e l'audio di Moonlight ha già un
  suo buffer più grande.
- Ogni aggiornamento di Sunshine richiede di riapplicare la patch (`scripts/sync-upstream.sh`).
