// Pagine dei link condivisi: aprono Falco con `falco://…`, altrimenti portano al download.
import { displayCode } from './auth';

const esc = (s: string) =>
  s.replace(/[<>&"']/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&#39;' })[c] ?? c);

function page(title: string, body: string, open: string | null, download: string): string {
  const btn = open ? `<a class="btn" href="${open}">Apri in Falco</a>` : '';
  // Con Falco installato il sistema lo apre subito
  const auto = open ? `<script>setTimeout(()=>{location.href=${JSON.stringify(open)}},300)</script>` : '';
  return `<!doctype html><html lang="it"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>${title} · Falco</title>
<style>
:root{color-scheme:dark;--bg:#07080e;--card:#131626;--line:rgba(255,255,255,.08);--text:#f1f3fa;--muted:#aab1c7;--accent:#4456c8}
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;padding:16px;background:var(--bg);color:var(--text);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif}
.card{width:min(420px,100%);display:grid;gap:16px;padding:32px;border:1px solid var(--line);border-radius:20px;background:var(--card);text-align:center}
h1{margin:0;font-size:22px;line-height:30px}p{margin:0;color:var(--muted)}
.beta{justify-self:center;padding:2px 8px;border-radius:999px;background:rgba(245,184,61,.14);color:#f5b83d;font-size:11px;font-weight:700;letter-spacing:.08em}
.btn{display:inline-flex;justify-content:center;align-items:center;height:48px;border-radius:10px;background:var(--accent);color:#fff;font-weight:600;text-decoration:none}
.alt{font-size:13px}.alt a{color:#9da9ff}
</style></head><body><main class="card"><span class="beta">BETA</span><h1>${title}</h1>${body}${btn}
<p class="alt">Non hai ancora Falco? <a href="${esc(download)}">Scaricalo gratis</a>, poi riapri questo link.</p>
</main>${auto}</body></html>`;
}

export const addPage = (name: string, code: string, download: string) =>
  page(
    `Aggiungi ${esc(name)}`,
    `<p>Codice amico <strong>${displayCode(code)}</strong>. Falco gli manderà una richiesta d'amicizia.</p>`,
    `falco://add/${code}`,
    download,
  );

export const joinPage = (name: string, code: string, download: string) =>
  page(
    `Guarda lo stream di ${esc(name)}`,
    '<p>Falco si collega da solo: niente account, niente configurazione.</p>',
    `falco://join/${code}`,
    download,
  );

export const missingPage = (download: string) =>
  page('Link non valido', '<p>Il link è scaduto o è stato revocato. Chiedine uno nuovo.</p>', null, download);
