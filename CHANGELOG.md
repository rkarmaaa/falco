# Changelog

Formato [Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni [SemVer](https://semver.org/lang/it/).

## [1.0.0-rc.10] - 2026-11-20 · Beta

### Corretto

- Amici sempre «offline» dopo che Tailscale ha dato al PC di uno dei due un nodo nuovo: Falco annuncia da solo il nodo nuovo con il profilo firmato, accettato solo se quello vecchio è spento ([ADR 0018](docs/adr/0018-nodo-tailscale-cambiato.md)). Serve la nuova versione su entrambi i PC.
- Condividere non ripartiva («porta 47989 già usata») se un Sunshine di Falco era rimasto aperto dopo un crash o un aggiornamento: ora viene chiuso.
- Falco si avviava con il sistema anche con l'opzione spenta (voce rimasta nel registro di sistema).
- Foto dell'amico mancante negli inviti a giocare.

### Migliorato

- Falco non si avvia più con il sistema di default: si attiva nelle Impostazioni. Aprendolo parte tutto ciò che serve.
- I popup si chiudono anche cliccando lo sfondo scuro.
- Nel rapporto di debug compare il motivo per cui un amico non risponde (per esempio «dispositivo non riconosciuto»).

## [1.0.0-rc.9] - 2026-11-20 · Beta

### Migliorato

- «Ospita» diventa **Condividi**: profili, opzioni principali e «Impostazioni avanzate» (host e streaming) in un posto solo; tolto il riquadro iniziale. Le schede Streaming, Host e Permessi escono dalle Impostazioni.
- Notifiche sopra lo stream in una finestra trasparente, senza togliere spazio al video; stesso stile per tutte, anche gli errori.
- Barra in alto a schermo intero più leggibile, con pulsante per nasconderla: riappare portando il cursore in alto.
- Scheda dell'amico in un popup centrale; Inviti con storico su richiesta (ultimi 10, al massimo 50 conservati per un mese).
- Ospiti nella barra in basso compatti: foto e dispositivi, nome e latenza al passaggio del mouse; avvisi della barra galleggianti; anelli di stato più tenui.
- Impostazioni: Sito web, GitHub e aggiornamenti come icone in alto a destra. Pulsante «Avvio…» mentre parte.
- Controller tradotti automaticamente: DualSense e DualShock diventano un DualShock 4, gli altri un Xbox 360.

### Corretto

- Tastiera dell'ospite: il focus torna sempre allo stream (ogni clic lo riportava alla pagina di Falco).
- Quando l'host chiude lo stream compare subito «Stream interrotto», senza messaggio d'errore.
- Finestre del terminale che lampeggiavano cambiando stream o permessi.
- Finestre di Moonlight che a volte comparivano fuori da Falco; video sempre contenuto nel palco.

## [1.0.0-rc.8] - 2026-11-20 · Beta

### Corretto

- Il controller dell'ospite non mandava comandi: nello stream incorporato SDL ignorava i gamepad («--background-gamepad»).
- «Invita» verso un amico già associato restava in caricamento: ora entra direttamente nella sessione, come quando chiede lui di entrare.

## [1.0.0-rc.7] - 2026-11-19 · Beta

### Aggiunto

- Controller degli ospiti pronti da subito: l'installer di Falco installa in silenzio il driver ViGEmBus (ufficiale, firmato da Nefarius) se manca. Nessun passaggio in più per l'utente.
- Menu dell'ospite: mentre un permesso viene applicato compare la rotella, poi la spunta.

## [1.0.0-rc.6] - 2026-11-19 · Beta

### Migliorato

- **Qualità dello stream** nella barra in basso mentre guardi: Leggera (720p 30, predefinita), Buona (1080p 60), Massima, Personalizzata; si ricollega da sola. Anche in Impostazioni → Streaming.
- «Statistiche» accende l'overlay di Moonlight sopra il video (fps, latenza, perdite) in tempo reale.
- Ospita → Personalizzato rifatto: scelte a pulsante spiegate (banda per ospite, protezione dalle perdite, nitidezza) e nuove opzioni «Ospiti insieme» e «Ruolo dei nuovi amici».
- «Ho un invito» rifatto: un solo campo per codice amico, link amico, link di stream o invito, con «Incolla».
- Pannello dell'amico più largo e scorrevole solo in verticale; «Controller» è un interruttore come gli altri permessi.
- Ospita → Dispositivi associati: «Può entrare adesso» / «Entra quando lo accetti» al posto di «Bloccato» e «Sblocca».
- Durante uno stream la barra in alto si attenua quando non ci passi sopra.

### Corretto

- La tastiera dell'ospite non arrivava all'host: il focus restava alla finestra di Falco invece che allo stream.
- Da spettatore il cursore spariva sopra il video.
- Bordino colorato attorno alle foto profilo; notifica di aggiornamento con lettere sbagliate («Ã¨»).

## [1.0.0-rc.5] - 2026-11-19 · Beta

### Aggiunto

- Permessi di tastiera, mouse e controller applicati **subito, senza riavviare lo stream**: Sunshine di Falco filtra gli input per ogni ospite (nuova patch `0002`). Niente più «Applica ora» né disconnessioni.
- Volume dello stream per chi guarda, con muto, nella barra in basso (solo l'audio dello stream, non quello del PC).
- Pulsante **Avvia** con interruttore acceso/spento al posto di «Condividi stream» e «Interrompi»; accanto, quando è attivo, il link da ricopiare.

### Corretto

- Si potevano aprire due finestre di Falco insieme: ora una seconda apertura porta in primo piano quella esistente.
- Mouse dell'ospite: passavano i clic ma non i movimenti (modalità assoluta nella finestra incorporata).
- Nella schermata Ospita comparivano tanti dispositivi con lo stesso nome (tentativi di pairing falliti): ora i duplicati si rimuovono da soli.

- Lo stream arrivava (60 fps) ma la finestra del video non compariva: Falco ora riconosce la finestra di Moonlight 6.2 (SDL3) e la cerca fino a 90 secondi; se non la trova, il motivo finisce nel report di debug.
- La foto profilo degli amici non compariva dopo l'aggiunta: ora si scarica anche se il profilo era già noto.

## [1.0.0-rc.4] - 2026-11-19 · Beta

### Aggiunto

- Nuovo sito **falcoapp.pages.dev**, con download per Windows (macOS in arrivo); i link amico e di stream usano il nuovo indirizzo.
- Impostazioni → Info: pulsanti «Sito web» e «GitHub».

### Corretto

- «Condividi» non partiva sui PC nuovi: Sunshine non poteva creare la sua cartella in Program Files. Ora la crea l'installer.
- Aggiornamento automatico bloccato dall'errore 740: ora l'installer si avvia chiedendo la conferma di Windows.
- L'associazione dei dispositivi non riusciva mai: Moonlight non poteva salvare il suo certificato in Program Files e a ogni avvio aveva un'identità nuova. Ora salva tutto nella cartella dell'utente.
- L'host conferma il PIN per 2 minuti invece di 30 secondi e non riavvia Sunshine durante un'associazione; i motivi di un fallimento finiscono nel log.

## [1.0.0-rc.3] - 2026-11-19 · Beta

### Aggiunto

- «Invita» su ogni amico: l'invito arriva come notifica con «Entra», anche a chi è offline (lo riceve appena apre Falco).

### Corretto

- Gli amici registrati sul server ma mai collegati a questo PC non comparivano in «Amici»; il gruppo «Offline» ora è aperto.
- Sunshine non partiva se il percorso della cartella utente aveva lettere accentate.
- Se Sunshine rifiuta la configurazione, l'errore riporta il suo messaggio invece di «il motore host non risponde».

- Se il motore di streaming dell'host non parte, chi entra riceve subito il motivo invece di aspettare due minuti, e l'invito resta valido.
- L'errore «il motore host non risponde» ora riporta il motivo scritto da Sunshine; «Condividi» riparte da capo al tentativo successivo.
- Un link di stream aperto due volte non consuma più l'invito con un secondo tentativo.
- Sulla rete Falco, se l'amico non è collegato alla rete l'errore arriva in un minuto e dice cosa fare.

## [1.0.0-rc.2] - 2026-11-18 · Beta

### Aggiunto

- **Falco Hub** (Cloudflare Workers, gratuito): amici per **codice** (`K7QM-2XPA`) o **link**, con richieste d'amicizia da accettare o rifiutare e notifica.
- **Condividi stream** con un clic: avvia la condivisione e crea un link corto; chi lo apre entra subito, senza account.
- **Rete Falco**: Tailscale si installa e si collega da solo con una chiave monouso, senza account per gli utenti. Le chiavi OAuth non servono più (restano facoltative in Rete per chi usa la propria tailnet).
- Badge **BETA**; nuovo selettore della foto profilo.
- Nuova interfaccia: barra in alto compatta, barra delle azioni e dello stream in basso, tema unico scuro «Notte».
- Lo stream resta dentro la finestra di Falco; «Schermo intero» con un pulsantino discreto per uscire.
- Ospiti collegati nella barra con tastiera, mouse e controller che si illuminano quando ricevono input; tasto destro per concedere i singoli dispositivi. Di default si può solo guardare.
- Qualità della condivisione: Bassa latenza, Bilanciato, Connessione lenta, Massima qualità, Personalizzato, e «Trova le impostazioni migliori».
- Ogni errore ha un messaggio chiaro con «Copia» e «Debug» (informazioni dettagliate copiabili).
- Audio condiviso «Solo ciò che è a schermo» (predefinito, Windows): gli ospiti sentono solo le app visibili sullo schermo condiviso, mai Discord e le altre chat vocali.

### Corretto

- Dopo il riavvio l'app poteva chiedere di nuovo nome e foto e poi dire «il profilo esiste già», bloccando la schermata di benvenuto.
- Avvisi in basso a destra: testi altissimi e pulsanti con spazio vuoto.
- Titoli e margini diversi tra Amici e Impostazioni: ora tutte le pagine usano lo stesso layout.
- Chiudere la finestra con la X mostrava l'errore «window|destroy not allowed by ACL».
- Dopo l'installazione la finestra diceva «Non riesco a parlare con Falco»: l'installer avviava il servizio con i privilegi di amministratore e la finestra non poteva collegarsi. Ora lo avvia come utente normale.

### Rimosso

- Tema chiaro e la scelta del tema (`general.theme` viene ignorato).

## [1.0.0-rc.1] - 2026-11-18

### Aggiunto

- **Falco** (prima Falò): nuovo nome, logo e palette con accento indaco `#32409A`.
- Host e client come Parsec: Sunshine dedicato e Moonlight portable pilotati dal core, pairing automatico.
- Identità firmata, amici, presenza peer-to-peer (FPP) solo dentro la tailnet.
- Inviti con un link `falco://`, anteprima dei permessi, rilevamento negli appunti.
- Permessi per amico applicati sull'host: ruoli, bussa, fasce orarie, scadenza, espulsione.
- Provider LAN (mDNS), Diretto e Headscale; Dottore di rete con banda, jitter, porte, NAT e postazione.
- Impostazioni complete del §15 generate dallo schema, esporta e importa.
- Aggiornamenti automatici firmati con notifica; installer Windows con firewall; Tailscale installato in un clic.

## [Non rilasciato]

### Aggiunto

- Monorepo: workspace Cargo (`apps/core`, `apps/ui/src-tauri`, `crates/*`) e UI Tauri 2 + Vue 3.
- `falco-core`: icona nel tray con stato, «Disponibile», «Apri Falco», «Esci»; server JSON-RPC su named pipe o socket Unix; istanza singola.
- `falco-proto`: identità Ed25519, profili firmati in JSON canonico (RFC 8785), ruoli e permessi del §7, link di invito firmati.
- `falco-store`: `config.toml` validato che conserva commenti e chiavi sconosciute; SQLite con migrazioni.
- `falco-net`: provider Tailscale via CLI e primi controlli del Dottore di rete.
- `falco-engines`: generazione di `sunshine.conf` e `apps.json`, argomenti e parametri automatici di Moonlight.
- UI: design system, schermate Amici (stato vuoto), Ospita, Inviti, Impostazioni con Dottore di rete; italiano e inglese; tema scuro e chiaro.
- Documentazione: README, DESIGN_SYSTEM, CONTRIBUTING, SECURITY, TODO, ADR.
