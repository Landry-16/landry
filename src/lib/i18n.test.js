import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import en from '../i18n/en.json';

// Every literal t('…') key used in the components must have an English translation.
describe('i18n', () => {
  it('has an English string for every key', () => {
    const dir = fileURLToPath(new URL('..', import.meta.url));
    const missing = [];
    for (const f of readdirSync(dir).filter((f) => f.endsWith('.svelte'))) {
      const src = readFileSync(dir + f, 'utf8');
      for (const m of src.matchAll(/\bt\(\s*(['"])((?:(?!\1).)*)\1/g)) {
        const key = m[2].replace(/\\n/g, '\n').replace(/\\'/g, "'").replace(/\\"/g, '"');
        if (!(key in en)) missing.push(`${f}: ${key}`);
      }
    }
    expect(missing).toEqual([]);
  });
});
