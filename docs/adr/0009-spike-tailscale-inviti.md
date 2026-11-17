# 0009 · Spike: inviti Tailscale con OAuth client

- **Stato**: proposto (da verificare)
- **Data**: 2026-11-17
- **Sezioni della specifica**: §6, §18, §19

## Da verificare

- [ ] OAuth client con scope minimi (`devices:core:read`, `device_invites`) e `POST /api/v2/device/{deviceId}/device-invites` con `multiUse` e `allowExitNode: false`.
- [ ] Accettazione automatica dell'invito inverso via API.
- [ ] Revoca via API di una condivisione già accettata.
- [ ] Rilevamento del completamento con `tailscale status --json` ogni 2 s.
- [ ] Serve un account Tailscale di prova: chiedere a Marco.
