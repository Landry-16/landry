import { describe, expect, it } from 'vitest';
import { cssVars, isDark, styleString } from './theme.js';

const preset = {
  theme: 'system',
  light: { bg: '#fff', fg: '#000', muted: '#666', accent: '#c50', selection: '#eee', border: '#ddd' },
  dark: { bg: '#111', fg: '#eee', muted: '#999', accent: '#fb4', selection: '#222', border: '#333' },
  font: { family: 'Inter', size: 15 },
  density: 'compact',
  opacity: 0.955,
  animation_ms: 0,
  layout: { anchor: 'top', width: 600, max_height: 400, offset_x: 0, offset_y: -20, margin: 50, radius: 8 },
  preview: { visible: true, side: 'right', width: 40 },
};

describe('theme', () => {
  it('follows the system only in system mode', () => {
    expect(isDark(preset, true)).toBe(true);
    expect(isDark(preset, false)).toBe(false);
    expect(isDark({ ...preset, theme: 'light' }, true)).toBe(false);
    expect(isDark({ ...preset, theme: 'dark' }, false)).toBe(true);
  });

  it('maps colors, layout and density', () => {
    const v = cssVars(preset, true);
    expect(v['--bg']).toBe('#111');
    expect(v['--width']).toBe('600px');
    expect(v['--offset-y']).toBe('-20px');
    expect(v['--opacity']).toBe('96%');
    expect(v['--row']).toBe('38px');
    expect(v['--anim']).toBe('0ms');
  });

  it('falls back to comfortable density', () => {
    expect(cssVars({ ...preset, density: 'unknown' }, false)['--row']).toBe('48px');
  });

  it('builds an inline style', () => {
    expect(styleString(preset, false)).toContain('--bg: #fff');
  });
});
