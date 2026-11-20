# ADR 0018 — Amico con un nodo Tailscale nuovo

- Stato: accettato
- Data: 2026-11-20

## Contesto

Il §5 impone un doppio vincolo su ogni richiesta FPP tra amici: firma dell'identità Falco **e** IP
sorgente che, risolto con `tailscale whois`, appartiene a un nodo registrato per quell'amico. I nodi
si registrano solo durante l'handshake di un invito (§6).

Tailscale può assegnare a uno stesso PC un nodo nuovo (reinstallazione, logout, chiave della rete
Falco scaduta): stesso nome con suffisso `-1`, `-2`, IP e ID stabile diversi. Successo a Marco e
Manuzzo il 2026-11-20: dal lato di chi ha ancora il nodo vecchio, l'amico interroga un IP spento;
dal lato di chi ha il nodo nuovo, ogni richiesta riceve 403 «dispositivo non riconosciuto».
Entrambi vedono l'altro offline, e il profilo firmato (che contiene i dispositivi) non si scarica
mai perché anche quella richiesta è rifiutata. L'unica uscita era un nuovo invito.

## Decisione

Nuovo endpoint `POST /v1/devices`. Chi riceve 403 dalla presenza di un amico gli invia il proprio
profilo firmato con il nodo attuale e la porta di presenza (`DeviceUpdate`). Il server registra il
nodo nuovo al posto dei vecchi solo se valgono **tutte** queste condizioni:

1. firma della richiesta valida e nella finestra di ±60 s, mittente già amico (come gli altri
   endpoint, senza il controllo sul nodo);
2. l'IP sorgente è della tailnet e `whois` lo risolve in un nodo;
3. quel nodo non è registrato per un altro amico;
4. il profilo allegato ha una firma valida, appartiene allo stesso ID che firma la richiesta ed
   elenca proprio quel nodo tra i dispositivi;
5. **nessuno dei nodi già registrati per l'amico è online** nella tailnet in quel momento: è una
   sostituzione, non un dispositivo in più. Se lo stato di Tailscale non si legge, si rifiuta.

Poi i vecchi nodi dell'amico si cancellano (si ricopiano le scelte di condivisione), si scrive un
avviso nel log e la lista amici si aggiorna. Il client riprova al massimo ogni 5 minuti, perché
Tailscale impiega qualche minuto a segnare offline il nodo vecchio.

Basta che uno dei due lati raggiunga l'altro: chi ha il nodo nuovo raggiunge ancora l'IP vecchio
dell'amico, si fa registrare e da lì la presenza riprende in entrambe le direzioni.

## Conseguenze

- Il doppio vincolo si indebolisce in un caso preciso: chi ruba la chiave d'identità di un amico
  **e** ha un nodo nella stessa tailnet può farsi registrare, ma solo mentre il PC vero è spento.
  Appena il PC vero torna online la sostituzione inversa è rifiutata finché il ladro resta online
  (la condizione 5 vale per tutti), quindi l'amico non si ripara da solo: serve un nuovo invito o
  rimuovere l'amico. Il log lo segnala. Con la sola rete Tailscale personale (non la rete Falco)
  chi non è nella tailnet non può comunque arrivare all'endpoint.
- Se cambiano nodo **entrambi** gli amici insieme, nessuno raggiunge l'altro: serve un nuovo
  invito, come prima.
- Con i provider LAN e Diretto non cambia nulla: lì l'IP fidato è quello annunciato o registrato.
