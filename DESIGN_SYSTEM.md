# Design system di Falco

> «Volo notturno»: superfici grafite fredde, un solo colore d'accento, **Cielo**, usato con parsimonia, tipografia compatta e decisa. Il falco vede tutto e arriva per primo: l'interfaccia è essenziale, rapida, senza ornamenti. Il colore indica sempre uno stato o un'azione, mai una decorazione.

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
| Nome | Sempre «Falco», con la maiuscola. Mai «FALCO» nei testi, mai abbreviato |
| Payoff | «Gioca con gli amici. Veloce come un falco.» (inglese: «Play with friends. Fast as a falcon.») |
| Simbolo | Testa di falco pellegrino di profilo, rivolta a destra (in avanti): cappuccio Cielo scuro con il «baffo» sotto l'occhio, guancia chiara, becco corto uncinato, occhio e cera **ambra**. File: [`assets/brand/falco-mark.svg`](assets/brand/falco-mark.svg) |
| Contenitore | Quadrato arrotondato (raggio 22% del lato) su `#12151B`, con alone Cielo tenue dietro la testa |
| Ambra dell'occhio | `$eye` `#F5A524`: solo nel simbolo e nelle illustrazioni, mai per pulsanti o stati |
| Area di rispetto | Almeno il 12% del lato attorno al contenitore |
| Dimensione minima | 16 px (favicon e tray); sotto i 24 px si usa il simbolo senza alone |
| Da non fare | Ruotare il falco a sinistra, cambiare i colori del simbolo, aggiungere testo dentro il contenitore |

Le icone dell'app si rigenerano dal simbolo con `npx tauri icon ../../assets/brand/falco-mark.svg -o src-tauri/icons` da `apps/ui`.

## Principi

1. **Una sola azione primaria per schermata**, sempre in Cielo con testo scuro.
2. **Gerarchia data da spazio e peso del testo**, non da riquadri: bordi da 1 px, nessuna ombra pesante.
3. **L'alone Cielo è solo per ciò che è «vivo»**: un amico in gioco, una sessione attiva, l'host disponibile.
4. **Tema scuro di default**, tema chiaro che segue il sistema; entrambi rispettano il contrasto WCAG AA.
5. **Lo stato non è mai indicato dal solo colore**: c'è sempre un testo o un'icona.
6. **Leggerezza**: nessuna libreria di componenti, nessun CSS-in-JS a runtime, animazioni solo su `transform` e `opacity`.

## Colori

I token SCSS (`$bg-0`, `$accent-500`…) puntano a custom property CSS (`var(--bg-0)`). Il tema cambia impostando `data-theme="dark|light"` su `<html>`; con `theme = "system"` lo decide `prefers-color-scheme`.

### Superfici e testo

| Token | Scuro | Chiaro | Uso |
| --- | --- | --- | --- |
| `$bg-0` | `#0B0D11` | `#F4F6F9` | Sfondo dell'app |
| `$bg-1` | `#12151B` | `#FFFFFF` | Barra laterale, pannelli, barra del titolo |
| `$bg-2` | `#1A1E26` | `#FAFBFD` | Card, campi |
| `$bg-3` | `#242A34` | `#E9EDF2` | Hover, elementi selezionati |
| `$line` | `#2E3540` | `#DCE2EA` | Bordi e separatori da 1 px |
| `$text-1` | `#EEF1F5` | `#12161C` | Testo principale (17,2:1) |
| `$text-2` | `#A7B0BD` | `#4E5967` | Testo secondario (8,9:1 / 6,6:1) |
| `$text-3` | `#8A93A0` | `#66707D` | Etichette, testo disattivato (6,3:1 / 4,6:1) |

### Cielo (accento)

| Token | Scuro | Chiaro | Uso |
| --- | --- | --- | --- |
| `$accent-500` | `#4CA6FF` | `#1F7FE0` | Azione primaria, sezione attiva, stato vivo |
| `$accent-400` | `#79BCFF` | `#3A93F0` | Hover |
| `$accent-600` | `#2B86E3` | `#1767BD` | Premuto |
| `$accent-glow` | `rgba(76,166,255,.35)` | `rgba(31,127,224,.28)` | Alone degli stati vivi |
| `$on-accent` | `#0B0D11` | `#0B0D11` | Testo e icone sopra l'accento (7,6:1 / 4,8:1) |
| `$eye` | `#F5A524` | `#D9890B` | Ambra dell'occhio: solo marchio e illustrazioni |

### Stati

| Token | Scuro | Chiaro | Significato |
| --- | --- | --- | --- |
| `$ok` | `#3DDC84` | `#1E9E5A` | Online, connessione diretta, ping buono |
| `$busy` | `#F5B83D` | `#B7791F` | Occupato, relay DERP, ping medio |
| `$danger` | `#FF4D5E` | `#D93848` | Errori, espulsione, ping alto |
| `$scrim` | `rgba(0,0,0,.55)` | `rgba(26,23,20,.35)` | Velo dietro modali e cassetti |

Tutte le coppie testo/sfondo superano WCAG AA in entrambi i temi (rapporti tra parentesi).

### Regole d'uso

- Il Cielo compare **al massimo una volta come riempimento** per schermata (l'azione primaria). Altrove solo come bordo, indicatore o alone.
- Verde, ambra e rosso sono riservati agli stati: mai usarli per decorare.
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
┌──────────────────────────────────────────────────────────────┐
│ FALCO                                    (barra del titolo 40) │
├──────┬───────────────────────────────────────┬───────────────┤
│  72  │ Area centrale                         │ Dettaglio 320 │
│ nav  │ padding 32/40 (24 sotto sm)           │ (cassetto     │
│      │                                       │  sotto sm)    │
│  ◉   │                                       │               │
└──────┴───────────────────────────────────────┴───────────────┘
```

| Elemento | Valore |
| --- | --- |
| Finestra | Minimo 960×600, default 1120×720 |
| Barra del titolo | 40 px, personalizzata e trascinabile; su macOS restano i semafori di sistema (80 px di rientro) |
| Barra laterale | 72 px: Amici, Ospita, Inviti, Impostazioni; in basso avatar e stato |
| Pannello destro | 320 px; sotto `sm` diventa un cassetto sovrapposto |
| Punti di rottura | `sm` 1040, `md` 1280, `lg` 1600 (larghezza finestra, mixin `mq`) |

## Componenti

| Componente | File | Varianti e regole |
| --- | --- | --- |
| Pulsante | `UiButton.vue` | `primary` (accento), `secondary` (superficie), `ghost`, `danger`; altezze 32, 40, 48; `icon` per pulsanti quadrati |
| Avatar | `UiAvatar.vue` | 24, 32, 40, 56, 96 px; anello di stato da 2 px con 2 px di stacco; monogramma sul colore `accent` se manca l'immagine; `playing` aggiunge l'alone |
| Interruttore | `UiSwitch.vue` | `role="switch"`, etichetta e suggerimento opzionale; altezza minima 40 |
| Scelta segmentata | `UiSegmented.vue` | `radiogroup` con frecce sinistra/destra; per 2–4 opzioni (tema, lingua, stato) |
| Stato vuoto | `EmptyState.vue` | Illustrazione, titolo Display, testo, un'azione |
| Riga diagnostica | `DiagnosticRow.vue` | Semaforo con icona, titolo, valore misurato o livello a parole, azione suggerita |
| Pillola ping | `PingPill.vue` | Verde < 30 ms, ambra 30–60 ms, rosso > 60 ms; etichetta «diretto» o «relay» |
| Barra del titolo | `TitleBar.vue` | Trascinabile, controlli finestra solo su Windows |
| Barra laterale | `SideBar.vue` | Icone 22 px, sezione attiva con fondo `$bg-3` e barretta accento |
| Errore core | `CoreOffline.vue` | Errore in tre parti, bordo `$danger`, `role="alert"` |

Da costruire nelle prossime milestone, con le regole già fissate dalla specifica:

| Componente | Regole |
| --- | --- |
| Scheda amico | Avatar, nome, attività, azioni visibili all'hover (sempre visibili sotto `sm`); bordo e alone accento quando l'amico è in gioco. Esempio SCSS nel §12 della specifica |
| Chip ruolo | Icona + nome del ruolo, colore neutro; tooltip con i permessi a parole |
| Campo, menu, slider | Altezza 40, raggio `$r-field`, focus con anello accento da 2 px |
| Toast | In basso a destra, massimo 2 visibili, 4 s, azione opzionale |
| Notifica aggiornamento | Toast persistente con «Aggiorna ora» (primario) e «Più tardi» (fantasma); badge accento su Impostazioni → Info |
| Modale | Raggio `$r-modal`, velo `$scrim`, entrata in `$t-modal` |

### Stati interattivi

| Stato | Resa |
| --- | --- |
| Hover | Fondo `$bg-3` oppure accento più chiara (`$accent-400`) |
| Premuto | `scale(.98)`; accento `$accent-600` |
| Focus da tastiera | Anello accento da 2 px con 2 px di stacco (`focus-ring`), solo con `:focus-visible` |
| Disattivato | Opacità 0,45, nessun evento |
| Vivo (in gioco, disponibile) | Bordo interno 1 px `$accent-500` e/o alone `glow` |

## Icone e illustrazioni

- Icone [Lucide](https://lucide.dev) da `lucide-vue-next`, importate una per una (`import { Users } from 'lucide-vue-next'`), tratto **1,75**, dimensioni 16, 18, 20 o 22.
- Le icone decorative hanno `aria-hidden="true"`; i pulsanti di sola icona hanno un'etichetta (`aria-label` o testo `sr-only`).
- Illustrazioni originali a tratto (vedi `FalconArt.vue`): falco in planata, tratto Cielo da 2,5, occhio ambra `$eye`, scie in `$text-3`, alone `$accent-glow` tenue.
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

- Approvazione del nuovo marchio e della palette «Volo notturno» (rinomina da Falò a Falco del 2026-11-18).
- Verifica che «Falco» non sia un marchio registrato nel settore del software di gioco.
- Rifinitura grafica delle schermate: prevista in una fase dedicata.
