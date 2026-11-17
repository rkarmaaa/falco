# 0010 · Spike: chiusura completa di WebView2

- **Stato**: proposto (da verificare)
- **Data**: 2026-11-17
- **Sezioni della specifica**: §10, §18

## Da verificare

- [ ] Chiudendo la finestra, `falo-ui.exe` termina e non restano processi `msedgewebview2.exe` figli.
- [ ] RAM del core prima e dopo un ciclo apri/chiudi (budget ≤ 15 MB).
- [ ] Tempo di apertura a freddo e a caldo (budget 800 ms e 300 ms).

La struttura a due processi (ADR 0001) è già pronta per la misura.
