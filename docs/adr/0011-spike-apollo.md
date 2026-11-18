# 0011 · Spike: permessi di Apollo su Sunshine

- **Stato**: proposto (da verificare)
- **Data**: 2026-11-17
- **Sezioni della specifica**: §7, §19

## Domanda

Portare il sistema di permessi per client di Apollo su Sunshine upstream, oppure usare Apollo come base dell'host?

## Da verificare

- [ ] Dimensione e punti di contatto della patch dei permessi in Apollo (sessione, negoziazione, input, avvio app).
- [ ] Distanza di Apollo da Sunshine upstream e frequenza dei rebase.
- [ ] Endpoint `PUT /api/falco/clients/{uuid}/permissions` e applicazione entro 100 ms.

Il modello dei permessi lato Falco è già pronto in `falco-proto::permissions` (ruoli, override, scadenza, fasce orarie).
