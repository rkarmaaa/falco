# 0012 · Nessuna installazione manuale per l'utente

- **Stato**: proposto (la scelta su Tailscale spetta a Marco)
- **Data**: 2026-11-18
- **Sezioni della specifica**: §1, §3, §9, §14, §19

## Contesto

Requisito di Marco: l'utente finale scarica Falco e lo usa subito, senza installare altre app a mano. La specifica originale chiedeva invece di installare Tailscale a parte. Sunshine e Moonlight erano già previsti dentro il pacchetto.

## Decisione

| Componente | Come arriva all'utente |
| --- | --- |
| `falco-sunshine`, `falco-moonlight` | Dentro il pacchetto di Falco (fork compilati in CI) |
| Driver dei controller virtuali (Windows) | Installato in silenzio dall'installer di Falco, come fa Sunshine |
| WebView2 (Windows) | Già presente in Windows 11; in mancanza, bootstrapper incluso nell'installer NSIS di Tauri |
| Tailscale | **Da decidere**, vedi sotto |

L'utente può dover solo confermare: la richiesta UAC dell'installer, i permessi di sistema su macOS, il login a Tailscale nel browser.

## Opzioni per Tailscale

**A · Installazione silenziosa del client ufficiale (consigliata per la v1)**
Falco scarica l'MSI o il pacchetto ufficiale, ne verifica la firma e lo installa in silenzio (`msiexec /qn` su Windows; su macOS la versione Standalone, con l'approvazione dell'estensione di rete che il sistema richiede comunque).
- Pro: è la rete WireGuard a livello kernel prevista dalla specifica, la latenza migliore. Il codice esistente (`falco-net`, CLI `--json`) resta valido.
- Contro: resta un'app Tailscale visibile nel sistema. Se l'utente ne ha già una, va riusata e non duplicata.

**B · `tsnet` incorporato in Falco**
La libreria Go ufficiale di Tailscale (BSD-3) gira dentro un processo di Falco, senza installare nulla.
- Pro: un solo pacchetto, nessuna app esterna, nessun conflitto con un Tailscale già installato.
- Contro: WireGuard in user space: più CPU e latenza da misurare con lo streaming a 50 Mbps. Serve un processo helper in Go che esponga il traffico a Sunshine e Moonlight (proxy locale o TUN). La specifica (§3) scelse apposta il livello kernel.

**C · Ibrido**: A di default, B come ripiego quando l'installazione non è possibile (account senza diritti di amministratore, criteri aziendali). Da valutare dopo la misura di B.

In tutti i casi resta un **account Tailscale** per utente, con login via Google, Microsoft o GitHub (§19). Eliminarlo richiederebbe un server di coordinamento nostro (Headscale), che contraddice «nessun server proprio».

## Da verificare

- [ ] Installazione silenziosa e firmata dell'MSI su Windows 11; riuso di un'installazione esistente.
- [ ] Esperienza su macOS: estensione di rete e richieste di sistema della versione Standalone.
- [ ] Spike `tsnet`: RTT e CPU a 1080p60 contro il client a livello kernel.
- [ ] Licenze e termini d'uso per la ridistribuzione o il download automatico del client.
