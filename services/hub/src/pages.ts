// Pagine pubbliche: home con i download e pagine dei link condivisi (aprono Falco con `falco://…`).
// Una schermata sola, senza scroll, stessi colori dell'app (tema «Notte»).
import { displayCode } from './auth';

const esc = (s: string) =>
  s.replace(/[<>&"']/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&#39;' })[c] ?? c);

const WINDOWS = `<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M3 5.1 10.4 4v7.1H3V5.1Zm8.4-1.2L21 2.5v8.6h-9.6V3.9ZM3 12.4h7.4v7.1L3 18.4v-6Zm8.4 0H21v8.6l-9.6-1.4v-7.2Z"/></svg>`;
const APPLE = `<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M16.4 12.6c0-2.5 2-3.6 2.1-3.7-1.2-1.7-3-1.9-3.6-2-1.5-.2-3 .9-3.8.9-.8 0-2-.9-3.3-.9-1.7 0-3.3 1-4.2 2.6-1.8 3.1-.5 7.7 1.3 10.2.8 1.2 1.8 2.6 3.1 2.6 1.3-.1 1.7-.8 3.3-.8 1.5 0 1.9.8 3.3.8 1.4 0 2.2-1.3 3-2.5.9-1.4 1.3-2.7 1.3-2.8-.1 0-2.5-1-2.5-4.4ZM14 5.2c.7-.8 1.2-2 1-3.2-1 .1-2.2.7-2.9 1.5-.6.7-1.2 1.9-1 3.1 1.1.1 2.2-.6 2.9-1.4Z"/></svg>`;

const STYLE = `
:root{color-scheme:dark;--bg:#07080e;--card:rgba(19,22,38,.72);--line:rgba(255,255,255,.08);--text:#f1f3fa;--muted:#aab1c7;--faint:#8d94ab;--accent:#4456c8;--accent-fg:#9da9ff;--glow:rgba(122,138,255,.38);--busy:#f5b83d}
*{box-sizing:border-box;margin:0}
html,body{height:100%}
body{display:grid;place-items:center;min-height:100dvh;padding:16px;overflow:hidden;background:var(--bg);color:var(--text);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif}
body::before{content:"";position:fixed;inset:-30% -10% auto;height:80%;background:radial-gradient(closest-side,rgba(68,86,200,.35),transparent 70%);pointer-events:none}
main{position:relative;width:min(440px,100%);display:grid;justify-items:center;gap:20px;padding:36px 32px;border:1px solid var(--line);border-radius:24px;background:var(--card);backdrop-filter:blur(18px);text-align:center;box-shadow:0 24px 64px rgba(0,0,0,.55)}
.logo{width:88px;height:88px;border-radius:22%;box-shadow:0 12px 40px var(--glow)}
.head{display:grid;justify-items:center;gap:8px}
.beta{padding:2px 9px;border-radius:999px;background:rgba(245,184,61,.14);color:var(--busy);font-size:11px;font-weight:700;letter-spacing:.08em}
h1{font-size:28px;line-height:34px;font-weight:700;letter-spacing:-.02em}
p{color:var(--muted)}
.actions{display:grid;gap:10px;width:100%}
.btn{display:flex;align-items:center;justify-content:center;gap:10px;height:52px;border-radius:12px;font-weight:600;text-decoration:none;color:#fff;background:linear-gradient(135deg,#4c5fd6,#32409a 55%,#263078);box-shadow:0 8px 24px rgba(50,64,154,.35);transition:transform .15s,filter .15s}
.btn:hover{transform:translateY(-1px);filter:brightness(1.08)}
.btn svg{width:20px;height:20px}
.btn.off{background:rgba(255,255,255,.04);border:1px solid var(--line);color:var(--faint);box-shadow:none;cursor:not-allowed}
.btn.off:hover{transform:none;filter:none}
.btn small{font-weight:500;opacity:.8}
.foot{font-size:13px;color:var(--faint)}
.foot a{color:var(--accent-fg);text-decoration:none}
strong{color:var(--text)}
`;

function shell(title: string, content: string, open: string | null = null): string {
  // Con Falco installato il sistema apre subito il link
  const auto = open ? `<script>setTimeout(()=>{location.href=${JSON.stringify(open)}},300)</script>` : '';
  return `<!doctype html><html lang="it"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>${title}</title>
<link rel="icon" href="/favicon.png"><meta name="theme-color" content="#07080e">
<meta property="og:title" content="${title}"><meta property="og:image" content="/logo.png">
<style>${STYLE}</style></head><body><main>${content}</main>${auto}</body></html>`;
}

const head = (title: string, body: string) =>
  `<img class="logo" src="/logo.png" alt="" width="88" height="88"><div class="head"><span class="beta">BETA</span><h1>${title}</h1>${body}</div>`;

const downloads = (download: string) => `<div class="actions">
<a class="btn" href="${esc(download)}">${WINDOWS}Scarica per Windows</a>
<span class="btn off" aria-disabled="true">${APPLE}macOS <small>· presto</small></span>
</div>`;

const footer = (repo: string) =>
  `<p class="foot">Gratis e open source · <a href="${esc(repo)}">GitHub</a></p>`;

export const homePage = (download: string, repo: string) =>
  shell(
    'Falco · Gioca in streaming con gli amici',
    head('Falco', '<p>Gioca in streaming con i tuoi amici, come sul divano accanto a te. Niente account, niente configurazione.</p>') +
      downloads(download) +
      footer(repo),
  );

const linkPage = (title: string, text: string, open: string, download: string, repo: string) =>
  shell(
    `${title} · Falco`,
    head(title, `<p>${text}</p>`) +
      `<div class="actions"><a class="btn" href="${open}">Apri in Falco</a></div>` +
      `<p class="foot">Non hai Falco? <a href="${esc(download)}">Scaricalo per Windows</a>, poi riapri questo link.</p>`,
    open,
  ).replace('</main>', `${footer(repo)}</main>`);

export const addPage = (name: string, code: string, download: string, repo: string) =>
  linkPage(
    `Aggiungi ${esc(name)}`,
    `Codice amico <strong>${displayCode(code)}</strong>. Falco gli manderà una richiesta d'amicizia.`,
    `falco://add/${code}`,
    download,
    repo,
  );

export const joinPage = (name: string, code: string, download: string, repo: string) =>
  linkPage(
    `Guarda lo stream di ${esc(name)}`,
    'Falco si collega da solo: niente account, niente configurazione.',
    `falco://join/${esc(code)}`,
    download,
    repo,
  );

export const missingPage = (download: string, repo: string) =>
  shell(
    'Link non valido · Falco',
    head('Link non valido', '<p>Il link è scaduto o è stato revocato. Chiedine uno nuovo.</p>') + downloads(download) + footer(repo),
  );
