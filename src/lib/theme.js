// Turns a preset into CSS custom properties. The only place where preset values meet CSS.

export const DENSITY = {
  compact: { pad: 6, gap: 2, row: 38 },
  comfortable: { pad: 10, gap: 4, row: 48 },
  spacious: { pad: 14, gap: 6, row: 58 },
};

export function isDark(preset, systemDark) {
  return preset.theme === 'dark' || (preset.theme === 'system' && systemDark);
}

export function cssVars(preset, systemDark) {
  const c = isDark(preset, systemDark) ? preset.dark : preset.light;
  const d = DENSITY[preset.density] ?? DENSITY.comfortable;
  const l = preset.layout;
  return {
    '--bg': c.bg,
    '--fg': c.fg,
    '--muted': c.muted,
    '--accent': c.accent,
    '--selection': c.selection,
    '--border': c.border,
    '--font': preset.font.family,
    '--font-size': `${preset.font.size}px`,
    '--opacity': `${Math.round(preset.opacity * 100)}%`,
    '--anim': `${preset.animation_ms}ms`,
    '--width': `${l.width}px`,
    '--max-height': `${l.max_height}px`,
    '--offset-x': `${l.offset_x}px`,
    '--offset-y': `${l.offset_y}px`,
    '--margin': `${l.margin}px`,
    '--radius': `${l.radius}px`,
    '--pad': `${d.pad}px`,
    '--gap': `${d.gap}px`,
    '--row': `${d.row}px`,
    '--preview-width': `${preset.preview.width}%`,
  };
}

/** Inline style string, for elements that preview a preset other than the active one. */
export function styleString(preset, systemDark) {
  return Object.entries(cssVars(preset, systemDark))
    .map(([k, v]) => `${k}: ${v}`)
    .join('; ');
}

export function applyToRoot(preset, systemDark, root = document.documentElement) {
  for (const [k, v] of Object.entries(cssVars(preset, systemDark))) root.style.setProperty(k, v);
  root.dataset.anchor = preset.layout.anchor;
  root.dataset.layer = preset.layout.layer;
  root.dataset.scheme = isDark(preset, systemDark) ? 'dark' : 'light';
}
