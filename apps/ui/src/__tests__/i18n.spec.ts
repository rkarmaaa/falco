import { describe, expect, it } from 'vitest';
import itMsgs from '@/i18n/locales/it.json';
import enMsgs from '@/i18n/locales/en.json';

const keys = (o: object, prefix = ''): string[] =>
  Object.entries(o).flatMap(([k, v]) =>
    typeof v === 'object' && v !== null ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );

describe('i18n', () => {
  it('italiano e inglese hanno le stesse chiavi', () => {
    expect(keys(enMsgs).sort()).toEqual(keys(itMsgs).sort());
  });
});
