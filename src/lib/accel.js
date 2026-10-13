// Keyboard combination <-> GNOME accelerator ("<Control><Alt>space").

const MODS = [
  ['ctrlKey', 'Control', 'Ctrl'],
  ['altKey', 'Alt', 'Alt'],
  ['shiftKey', 'Shift', 'Maj'],
  ['metaKey', 'Super', 'Super'],
];
const NAMES = { ' ': 'space', Enter: 'Return', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', ',': 'comma', '.': 'period', ';': 'semicolon', '/': 'slash', '-': 'minus', '=': 'equal' };
const LABELS = { space: 'Espace', Return: 'Entrée', Up: '↑', Down: '↓', Left: '←', Right: '→', comma: ',', period: '.', semicolon: ';', slash: '/', minus: '-', equal: '=' };

/** GNOME accelerator for a keydown event, or null while only modifiers are held or none is. */
export function fromEvent(e) {
  const key = NAMES[e.key] ?? (/^F\d{1,2}$/.test(e.key) ? e.key : /^[a-z0-9]$/i.test(e.key) ? e.key.toLowerCase() : null);
  const mods = MODS.filter(([k]) => e[k]).map(([, g]) => `<${g}>`).join('');
  // A plain letter would fire while typing anywhere: a modifier is required, except for F keys.
  if (!key || (!mods && !key.startsWith('F'))) return null;
  return mods + key;
}

/** "<Control><Alt>space" -> "Ctrl + Alt + Espace". */
export function label(accel) {
  if (!accel) return '';
  const mods = [...accel.matchAll(/<(\w+)>/g)].map(([, m]) => MODS.find(([, g]) => g === m || (m === 'Primary' && g === 'Control'))?.[2] ?? m);
  const key = accel.replace(/<\w+>/g, '');
  return [...mods, LABELS[key] ?? (key.length === 1 ? key.toUpperCase() : key)].join(' + ');
}
