import { describe, expect, it } from 'vitest';
import { fromEvent, label } from './accel.js';

const ev = (key, mods = {}) => ({ key, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });

describe('accel', () => {
  it('turns a combination into a GNOME accelerator', () => {
    expect(fromEvent(ev(' ', { ctrlKey: true, altKey: true }))).toBe('<Control><Alt>space');
    expect(fromEvent(ev('Enter', { metaKey: true }))).toBe('<Super>Return');
    expect(fromEvent(ev('K', { ctrlKey: true, shiftKey: true }))).toBe('<Control><Shift>k');
    expect(fromEvent(ev('F12'))).toBe('F12');
  });

  it('waits for a real key and refuses a bare letter', () => {
    expect(fromEvent(ev('Control', { ctrlKey: true }))).toBeNull();
    expect(fromEvent(ev('a'))).toBeNull();
  });

  it('reads accelerators back', () => {
    expect(label('<Control><Alt>space')).toBe('Ctrl + Alt + Espace');
    expect(label('<Primary>k')).toBe('Ctrl + K');
    expect(label('')).toBe('');
  });
});
