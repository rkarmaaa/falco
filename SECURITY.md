# Sicurezza

## Segnalare una vulnerabilità

Non aprire una issue pubblica. Usa la funzione **«Report a vulnerability»** nella scheda *Security* del repository GitHub (segnalazione privata). Descrivi il problema, i passi per riprodurlo e l'impatto. Rispondiamo entro 7 giorni.

Le vulnerabilità di Sunshine, Moonlight o Tailscale vanno segnalate ai rispettivi progetti; se riguardano anche i fork di Falò, avvisaci pure.

## Modello di sicurezza in breve

Il modello completo è nel §11 della [specifica](docs/falo-spec.md).

- Nessuna porta di Falò o di Sunshine è raggiungibile da Internet: il server di presenza ascolta solo sugli indirizzi Tailscale, il firewall limita Sunshine agli intervalli `100.64.0.0/10` e `fd7a:115c:a1e0::/48`.
- Ogni richiesta tra amici è firmata con Ed25519, con finestra temporale di ±60 s, verifica `tailscale whois` e limite di frequenza.
- I permessi si applicano sull'host, nel fork di Sunshine: un client modificato non ottiene più di quanto concesso.
- L'IPC locale tra core e UI usa una named pipe accessibile solo all'utente corrente (Windows) o un socket `0600` (macOS).
- I segreti (chiave privata, OAuth Tailscale, credenziali di Sunshine) stanno nel portachiavi di sistema; dei token d'invito si salva solo lo SHA-256.
- Gli aggiornamenti sono firmati e verificati prima dell'installazione.
- Nessuna telemetria.

## Versioni supportate

Prima della 1.0 riceve correzioni solo l'ultima versione pubblicata.
