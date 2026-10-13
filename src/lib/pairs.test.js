import { describe, expect, it } from 'vitest';
import { formatPairs, parsePairs } from './pairs.js';

describe('pairs', () => {
  it('parses lines, keeps "=" inside values, skips comments and blanks', () => {
    const r = parsePairs('nc = nacre\n\n# commentaire\ng = https://www.google.com/search?q={q}&hl=fr');
    expect(r.errors).toEqual([]);
    expect(r.map).toEqual({ nc: 'nacre', g: 'https://www.google.com/search?q={q}&hl=fr' });
  });

  it('reports invalid line numbers', () => {
    expect(parsePairs('ok = 1\nsans egal\n= vide').errors).toEqual([2, 3]);
  });

  it('round-trips', () => {
    const m = { a: 'b', c: 'd = e' };
    expect(parsePairs(formatPairs(m)).map).toEqual(m);
  });
});
