// Crea latest.json per l'updater (formato di Tauri, ADR 0008) dagli artefatti firmati.
// Uso: node scripts/make-manifest.mjs <versione> <url-base-del-rilascio> <cartella-artefatti> [note]
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const [version, baseUrl, dir, notes = ''] = process.argv.slice(2);
if (!version || !baseUrl || !dir) {
  console.error('uso: make-manifest.mjs <versione> <url-base> <cartella> [note]');
  process.exit(1);
}

// Artefatto → piattaforma dell'updater
const rules = [
  [/x64-setup\.exe$/, 'windows-x86_64'],
  [/aarch64\.app\.tar\.gz$/, 'darwin-aarch64'],
  [/x64\.app\.tar\.gz$/, 'darwin-x86_64'],
];

const platforms = {};
for (const file of readdirSync(dir)) {
  const rule = rules.find(([rx]) => rx.test(file));
  if (!rule) continue;
  const sigFile = path.join(dir, `${file}.sig`);
  platforms[rule[1]] = {
    signature: readFileSync(sigFile, 'utf8').trim(),
    url: `${baseUrl.replace(/\/$/, '')}/${encodeURIComponent(file)}`,
  };
}

if (Object.keys(platforms).length === 0) {
  console.error('nessun artefatto firmato trovato');
  process.exit(1);
}
const manifest = { version: version.replace(/^v/, ''), notes, pub_date: new Date().toISOString(), platforms };
writeFileSync(path.join(dir, 'latest.json'), JSON.stringify(manifest, null, 2));
console.log(`latest.json: ${Object.keys(platforms).join(', ')}`);
