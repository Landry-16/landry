import { describe, expect, it } from 'vitest';
import { contrast } from './contrast.js';

describe('contrast', () => {
  it('matches WCAG reference values', () => {
    expect(contrast('#fff', '#000')).toBeCloseTo(21, 1);
    expect(contrast('#767676', '#ffffff')).toBeCloseTo(4.54, 2);
    expect(contrast('#ffffff', '#ffffff')).toBe(1);
  });
  it('rejects invalid colors', () => {
    expect(contrast('rouge', '#fff')).toBeNull();
  });
});
