// "key = value" lines <-> object, for aliases and search prefixes.

export function parsePairs(text) {
  const map = {};
  const errors = [];
  text.split('\n').forEach((line, i) => {
    const l = line.trim();
    if (!l || l.startsWith('#')) return;
    const eq = l.indexOf('=');
    const key = eq > 0 ? l.slice(0, eq).trim() : '';
    const value = eq > 0 ? l.slice(eq + 1).trim() : '';
    if (!key || !value) errors.push(i + 1);
    else map[key] = value;
  });
  return { map, errors };
}

export function formatPairs(obj) {
  return Object.entries(obj ?? {})
    .map(([k, v]) => `${k} = ${v}`)
    .join('\n');
}
