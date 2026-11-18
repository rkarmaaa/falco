# Design system di Falco

> «Volo notturno»: superfici blu notte, un solo colore d'accento, l'**indaco Falco** del logo, usato con parsimonia, tipografia compatta e decisa. Il falco vede tutto e arriva per primo: l'interfaccia è essenziale, rapida, senza ornamenti. Il colore indica sempre uno stato o un'azione, mai una decorazione.

Questo documento è la guida pratica al design system. La fonte di verità per i valori resta il §12 di [`docs/falco-spec.md`](docs/falco-spec.md); l'implementazione vive in [`apps/ui/src/styles`](apps/ui/src/styles) e nei componenti di [`apps/ui/src/components`](apps/ui/src/components). Se un file Figma delle schermate esiste, è la fonte del layout; colori, spazi e tipografia vengono sempre da qui.

## Indice

1. [Marchio](#marchio)
2. [Principi](#principi)
3. [Colori](#colori)
4. [Tipografia](#tipografia)
5. [Spazi, raggi, movimento](#spazi-raggi-movimento)
6. [Struttura della finestra](#struttura-della-finestra)
7. [Componenti](#componenti)
8. [Icone e illustrazioni](#icone-e-illustrazioni)
9. [Testi e tono](#testi-e-tono)
10. [Accessibilità](#accessibilità)
11. [Regole SCSS](#regole-scss)
12. [Questioni aperte](#questioni-aperte)

## Marchio

**Falco** — gioca in streaming con gli amici, *veloce come un falco*.

| Elemento | Regola |
| --- | --- |
| Nome | Sempre «Falco», con la maiuscola. Mai abbreviato |
| Payoff | «Gioca con gli amici. Veloce come un falco.» (inglese: «Play with friends. Fast as a falcon.») |
| Simbolo | Testa di rapace di profilo in retino (mezzatinta) bianco e nero su gradiente indaco, disegnato da Marco. Sorgente: [`assets/brand/falco-icon.svg`](assets/brand/falco-icon.svg) |
| Gradiente | Da `#32409A` (in alto a sinistra) a `#111634` (in basso a destra); raggio degli angoli 96/512 |
| Colore del brand | `#32409A`, preso dal punto più chiaro dello sfondo: è l'accento dell'interfaccia |
| Dimensione minima | 16 px (favicon e tray) |
| Da non fare | Ricolorare il retino, specchiare il simbolo, aggiungere testo dentro il contenitore |

**Formati pronti** in `assets/brand/`:

| File | Uso |
| --- | --- |
| `falco-icon.svg` | Sorgente vettoriale con il retino incorporato |
| `png/falco-{16,24,32,48,64,128,256,512,1024}.png` | Siti, store, documenti, social |
| `falco.ico` | Windows (multi-risoluzione) |
| `falco.icns` | macOS |
| `apps/ui/src-tauri/icons/*` | Icone del pacchetto e del tray (generate con Tauri) |
| `apps/ui/public/favicon.ico`, `favicon-32.png` | Favicon della UI |

Per rigenerare tutto dal sorgente: `npx tauri icon ../../assets/brand/falco-icon.svg -o src-tauri/icons` da `apps/ui`, e con `-p 16,24,32,48,64,128,256,512,1024` per i PNG.

## Principi

1. **Una sola azione primaria per schermata**, sempre in indaco con testo bianco.
2. **Gerarchia data da spazio e peso del testo**, non da riquadri: bordi da 1 px, nessuna ombra pesante.
3. **Il verde «vivo» è solo per ciò che sta succedendo adesso**: condivisione attiva, sessione connessa, input che arriva. L'indaco è per le azioni e la selezione.
7. **Le azioni stanno in basso**, sempre nello stesso posto: la barra delle azioni cambia con il contesto (niente, condivisione, stream) ma non si sposta mai.
4. **Un solo tema, scuro: «Notte»**. Cielo notturno con un alone indaco dall'alto a sinistra, come nel logo; contrasto WCAG AA.
5. **Lo stato non è mai indicato dal solo colore**: c'è sempre un testo o un'icona.
6. **Leggerezza**: nessuna libreria di componenti, nessun CSS-in-JS a runtime, animazioni solo su `transform` e `opacity`.

## Colori

I token SCSS (`$bg-0`, `$accent-500`…) puntano a custom property CSS (`var(--bg-0)`) definite una volta in `_base.scss` dalla mappa `$palette`. Non c'è tema chiaro.

### Superfici e testo

| Token | Valore | Uso |
| --- | --- | --- |
| `$bg-0` | `#07080E` | Fondo dell'app, sotto l'alone indaco |
| `$bg-1` | `#0B0D17` | Barre |
| `$bg-2` | `rgba(19,22,38,.72)` | Card e pannelli, leggermente traslucidi |
| `$bg-3` | `#191D31` | Hover, elementi aperti |
| `$bg-4` | `#22273F` | Elemento selezionato, icone su card |
| `$line` / `$line-strong` | bianco al 6% / 11% | Bordi da 1 px; quello forte per menu e avvisi |
| `$text-1` | `#F1F3FA` | Testo principale |
| `$text-2` | `#AAB1C7` | Testo secondario |
| `$text-3` | `#8D94AB` | Etichette, suggerimenti (≥ 4,5:1 su `$bg-2`) |

### Accento (indaco Falco)

| Token | Valore | Uso |
| --- | --- | --- |
| `$accent-500` | `#32409A` | Riempimenti: azione primaria (testo bianco) |
| `$accent-400` / `$accent-600` | `#4456C8` / `#283381` | Hover / premuto |
| `$accent-fg` | `#9DA9FF` | Testo, bordi, focus, note |
| `$accent-soft` | indaco al 14% | Fondo di elementi attivi (profilo scelto, consiglio, interruttore premuto) |
| `$grad-accent` | `#4C5FD6 → #32409A → #263078` | Icona del profilo scelto, elementi di marca |

### Vivo e stati

| Token | Valore | Significato |
| --- | --- | --- |
| `$live` | `#46E0A9` | In condivisione, connesso, input in arrivo |
| `$live-soft` / `$live-glow` | verde al 14% / 45% | Fondo e alone di ciò che è vivo |
| `$ok` | `#46E0A9` | Rete pronta, ping buono |
| `$busy` | `#F5B83D` | Occupato, relay, ping medio, modifiche in attesa |
| `$danger` / `$danger-soft` | `#FF5C6C` / al 14% | Errori, «Esci», «Interrompi», «Espelli» |
| `$scrim` | `rgba(3,4,9,.66)` | Velo dietro le modali |

### Regole d'uso

- L'indaco pieno compare **una volta per zona** (l'azione primaria della pagina, quella della barra in basso).
- Il verde vivo non decora mai: se è verde, sta succedendo qualcosa adesso.
- Il colore `accent` di un amico (derivato dall'hash del suo ID) serve solo al suo monogramma.

## Tipografia

Un solo font variabile, [Geist](https://vercel.com/font) (licenza OFL), sottoinsieme latino in WOFF2 (circa 29 KB, sotto il budget di 50 KB), con lo stack di sistema come ripiego. Pacchetto: `@fontsource-variable/geist`, caricato solo il file `geist-latin-wght-normal.woff2`.

| Stile | Token | Dimensione / interlinea | Peso | Uso |
| --- | --- | --- | --- | --- |
| Display | `$fs-display` | 32 / 38 | 700, spaziatura −0,02em | Onboarding, schermate vuote |
| Titolo 1 | `$fs-h1` | 24 / 30 | 650 | Titolo di pagina |
| Titolo 2 | `$fs-h2` | 18 / 24 | 600 | Sezioni, nome nella scheda amico |
| Corpo | `$fs-body` | 14 / 20 | 450 | Testo standard |
| Piccolo | `$fs-small` | 13 / 18 | 450 | Testo secondario |
| Etichetta | `$fs-label` | 11 / 14 | 600, maiuscolo, +0,06em | Intestazioni di gruppo (mixin `label`) |

I numeri delle statistiche (ping, fps, bitrate) usano `font-variant-numeric: tabular-nums`, così non «ballano».

## Spazi, raggi, movimento

| Gruppo | Token | Valori |
| --- | --- | --- |
| Spaziatura (base 4) | `$sp-1` … `$sp-9` | 4, 8, 12, 16, 20, 24, 32, 40, 56 |
| Raggi | `$r-field`, `$r-button`, `$r-card`, `$r-modal`, `$r-pill` | 6, 10, 14, 20, 999 |
| Durate | `$t-fast`, `$t-panel`, `$t-modal` | 120 ms hover, 200 ms pannelli, 320 ms modali |
| Curva | `$ease` | `cubic-bezier(.2, .8, .2, 1)` |
| Altezza controlli | `$control-h` | 40 |

Con `prefers-reduced-motion: reduce` le tre durate diventano 0 ms: basta usare i token per rispettarlo.

## Struttura della finestra

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ ◆ Falco  [● Luca][Amici][Ospita][Inviti][Impostazioni]   ● Rete pronta – □ ×│  barra in alto 48
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   Contenuto della sezione (max 1120, padding 32/40)                      │
│   oppure il palco dello stream (padding 12, bordo arrotondato)           │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ (M) Marco      │ [L Luca ⌨ 🖱 🎮] [B Bea 👁]          Profilo ▾ Invita [Interrompi]│  barra azioni 72
│     In condiv. │                                                         │
└──────────────────────────────────────────────────────────────────────────┘
```

| Elemento | Valore |
| --- | --- |
| Finestra | Minimo 960×600, default 1120×720 |
| Barra in alto | 48 px, trascinabile: marchio, sezioni a pillola, scheda verde della sessione in corso, stato della rete, controlli finestra (su macOS i semafori, 84 px di rientro) |
| Barra delle azioni | 72 px, sempre visibile tranne a schermo intero. Da sinistra: card utente; ospiti collegati (se condividi) o la sessione che guardi (se sei ospite); azioni; in fondo a destra «Schermo intero» durante uno stream |
| Card utente | Avatar 40, nome, sotto l'ID (con «Copia») oppure lo stato: «In condivisione» in verde, «Offline», «Occupato». Clic: stato e profilo |
| Palco | Lo stream di Moonlight incorporato ([ADR 0015](docs/adr/0015-stream-incorporato-e-input-per-dispositivo.md)); gli avvisi gli si affiancano a destra (340 px) invece di coprirlo |
| Schermo intero | Solo lo stream; in alto al centro il pulsantino `StreamPill` (opacità 0,28 a riposo, piena al passaggio) |
| Pannello destro | 320 px; sotto `sm` diventa un cassetto sovrapposto |
| Punti di rottura | `sm` 1040, `md` 1280, `lg` 1600 (larghezza finestra, mixin `mq`) |

## Componenti

| Componente | File | Varianti e regole |
| --- | --- | --- |
| Barra in alto | `TopBar.vue` | Sezioni a pillola; la sessione in corso appare come prima scheda, verde, con punto pulsante |
| Barra delle azioni | `ActionBar.vue` | Tre contesti: inattivo («Condividi» primario), condivisione (profilo, «Invita», «Interrompi»), stream («Statistiche», «Esci», «Schermo intero» primario) |
| Card utente | `UserCard.vue` | Vedi sopra; menu di stato con `menuitemradio` |
| Ospite | `GuestChip.vue` | Avatar 32, nome, icone dei dispositivi o «Guarda»; tasto destro o clic (anche Maiusc+F10) apre il menu: dispositivi, «Solo guardare», «Espelli» |
| Icone dispositivi | `DeviceIcons.vue` | Tastiera, mouse, controller: spente (38%) se non concessi, accese se concessi, verdi con alone mentre arriva input; stato anche nel testo |
| Menu contestuale | `ContextMenu.vue` | Si apre nel punto del clic e resta nella finestra; frecce, Esc, clic fuori; vetro scuro con `$shadow-3`; copre il palco |
| Palco | `StreamStage.vue` | Avanzamento a tre passi (bussa, connessione, video) con alone rotante sull'avatar dell'host |
| Pulsantino | `StreamPill.vue` | Nome, ping, «Esci da schermo intero», «Esci» con conferma al secondo clic |
| Qualità della condivisione | `ShareSettings.vue` | Cinque card a scelta singola (`radio`); «Trova le impostazioni migliori» misura la rete e consiglia un profilo con i motivi |
| Avvisi | `ToastStack.vue` | Richieste (bussa, inviti a giocare, amicizie) con bordo indaco; errori con Copia e Debug; messaggi brevi con un'azione |
| Errore | `ErrorNotice.vue`, `DebugDialog.vue` | Messaggio in tre parti, «Copia», «Debug» con sezioni copiabili |
| Pulsante | `UiButton.vue` | `primary`, `secondary`, `ghost`, `danger`; altezze 32, 40, 48 |
| Avatar | `UiAvatar.vue` | 24, 32, 40, 56, 96 px; anello di stato; monogramma sul colore `accent` |
| Interruttore, scelta segmentata, campo | `UiSwitch.vue`, `UiSegmented.vue`, `UiField.vue` | Altezza 40, focus con anello accento |
| Modale | `UiModal.vue` | `<dialog>` nativo, raggio `$r-modal`, velo `$scrim`; copre il palco |
| Riga diagnostica | `DiagnosticRow.vue` | Semaforo con icona, valore misurato, azione suggerita |
| Stato vuoto | `EmptyState.vue` | Illustrazione, titolo, testo, un'azione |

### Stati interattivi

| Stato | Resa |
| --- | --- |
| Hover | Fondo `$bg-3` oppure accento più chiara (`$accent-400`) |
| Premuto | `scale(.98)`; accento `$accent-600` |
| Focus da tastiera | Anello accento da 2 px con 2 px di stacco (`focus-ring`), solo con `:focus-visible` |
| Disattivato | Opacità 0,45, nessun evento |
| Vivo (in condivisione, connesso, input) | Bordo verde al 30%, fondo `$live-soft`, alone `$live-glow` |

## Icone e illustrazioni

- Icone [Lucide](https://lucide.dev) da `lucide-vue-next`, importate una per una (`import { Users } from 'lucide-vue-next'`), tratto **1,75**, dimensioni 16, 18, 20 o 22.
- Le icone decorative hanno `aria-hidden="true"`; i pulsanti di sola icona hanno un'etichetta (`aria-label` o testo `sr-only`).
- Illustrazioni originali a tratto (vedi `FalconArt.vue`): falco in planata, tratto `$accent-fg` da 2,5, occhio `$eye`, scie in `$text-3`, alone `$accent-glow` tenue.
- Nessun asset di Parsec, Moonlight o Sunshine.

## Testi e tono

- Tono informale, con il tu, frasi brevi. Nessun termine tecnico nelle schermate di base; i termini tecnici stanno nelle impostazioni avanzate.
- Ogni testo passa da una chiave i18n in [`apps/ui/src/i18n/locales`](apps/ui/src/i18n/locales), in italiano e in inglese. Un test verifica che le due lingue abbiano le stesse chiavi.
- **Errori sempre in tre parti**: cosa è successo, perché probabilmente, cosa fare. Esempio: «Non riesco a raggiungere Marco. Il suo Tailscale sembra spento. Chiedigli di aprire Falco.»
- Pulsanti con verbi all'imperativo e oggetto chiaro: «Invita il primo», «Ripeti il test», «Aggiorna ora».

## Accessibilità

- Tutto si usa da tastiera, con focus sempre visibile; aree cliccabili di almeno 32 px.
- Ruoli ARIA corretti: `switch`, `radiogroup`, `tablist`, `alert`, `aria-current` sulla sezione attiva, `aria-live` per conferme e toast.
- Contrasto AA su entrambi i temi (vedi la questione aperta su `$text-3`).
- `prefers-reduced-motion` rispettato tramite i token di durata.

## Regole SCSS

- File globali in `apps/ui/src/styles`: `_tokens.scss` (variabili e mappe), `_mixins.scss`, `_base.scss` (temi, reset, font), `main.scss`. Token e mixin sono iniettati in ogni componente da Vite: non serve importarli.
- Ogni componente Vue usa `<style lang="scss" scoped>` e parte dalla sua classe radice, con tutto annidato sotto.
- Usare sempre token e mixin esistenti, mai valori scritti a mano se esiste un token.
- Le regole responsive stanno dentro il singolo selettore annidato, con `@include mq(sm)`.
- Nomi di classe semplici, senza `--` e `__`: `.friend`, `.name`, `.actions`; stati come `.playing`, `.active`, `.on`.
- Commenti brevi, massimo due righe.

### Mixin

| Mixin | Effetto |
| --- | --- |
| `mq($bp)` | Media query per larghezza finestra sotto `sm`, `md` o `lg` |
| `surface($level)` | Fondo `$bg-1/2/3` con bordo `$line` da 1 px |
| `focus-ring` | Anello accento su `:focus-visible` |
| `truncate` | Testo su una riga con puntini |
| `glow($color)` | Alone per gli stati vivi |
| `avatar($size)` | Cerchio di dimensione fissa |
| `label` | Stile Etichetta (11 px, maiuscolo) |
| `reset-button` | Toglie gli stili nativi del `<button>` |

### Esempio

```scss
// Card amico: stato, attività, azioni rapide
.friends {
  .friend {
    @include surface(2);
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: $sp-3;
    padding: $sp-3 $sp-4;
    border-radius: $r-card;

    .actions {
      opacity: 0;
      transition: opacity $t-fast $ease;

      @include mq(sm) {
        opacity: 1;
      }
    }

    &:hover .actions {
      opacity: 1;
    }

    // In gioco: bordo accento e alone sull'avatar
    &.playing {
      box-shadow: inset 0 0 0 1px $accent-500;

      .avatar {
        @include glow($accent-glow);
      }
    }
  }
}
```

## Questioni aperte

- Approvazione delle schermate da parte di Marco (§ CLAUDE.md).
- Illustrazioni per gli stati vuoti in stile «Notte».
- Verifica che «Falco» non sia un marchio registrato nel settore del software di gioco.
