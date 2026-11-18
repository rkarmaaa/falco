import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// JSON grezzi: l'import passerebbe dal compilatore dei messaggi
const load = (lang: string) =>
  JSON.parse(readFileSync(resolve(process.cwd(), `src/i18n/locales/${lang}.json`), 'utf8'));

const keys = (o: object, prefix = ''): string[] =>
  Object.entries(o).flatMap(([k, v]) =>
    typeof v === 'object' && v !== null ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );

describe('i18n', () => {
  it('italiano e inglese hanno le stesse chiavi', () => {
    expect(keys(load('en')).sort()).toEqual(keys(load('it')).sort());
  });

  it('nessun testo vuoto', () => {
    for (const lang of ['it', 'en']) {
      const flat = keys(load(lang));
      expect(flat.length).toBeGreaterThan(100);
    }
  });
});
