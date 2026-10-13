<script>
  // The vouivre, Landry's companion: a pixel winged serpent whose mood follows the bar.
  // Same sprite and reactions as the Figma file "Landry - Direction artistique", component Vouivre.
  let { mood = 'idle' } = $props();

  const SPRITE = [
    '...........h..h..',
    '...........hh.h..',
    '.w.w......HHHHH..',
    '.wWww....HHHHHHHH',
    'wWwWw....HHHHHHHf',
    '.wwWwww.cHHHbbb..',
    '..wwwwwcBBb......',
    '...ww.cBBb.......',
    '.....cBBb........',
    '..cBBBBb.........',
    '.cBBbbbb.........',
    'cBb.........s....',
    'cBbb.......sss...',
    '.cBBBbbbbbBBs....',
    '...cBBBBBBBs.....',
  ];
  const FILL = { H: 'scale', B: 'scale', c: 'shade', b: 'belly', h: 'horn', w: 'wing', W: 'vein', s: 'wing', f: 'tongue' };
  const layerOf = (ch, r, c) => ('wW'.includes(ch) ? 'wing' : ch === 's' ? 'tail' : r <= 5 && c >= 9 ? 'head' : 'body');
  const LAYERS = { wing: [], body: [], tail: [], head: [] };
  SPRITE.forEach((row, r) => [...row].forEach((ch, c) => FILL[ch] && LAYERS[layerOf(ch, r, c)].push({ x: c, y: r, k: FILL[ch] })));

  // Eye pixels [row, col], and what appears around: s sparks or signs, f flame, m smoke.
  const MOODS = {
    arrive: { eye: [[3, 12]] },
    idle: { eye: [[3, 12]] },
    listen: { eye: [[3, 13]], signs: [[0, 17], [-1, 18], [2, 18]] },
    point: { eye: [[4, 12]] },
    lost: { eye: [[3, 12]], quiet: true, signs: [[-3, 6], [-3, 7], [-2, 8], [-1, 7], [1, 7]] },
    joy: { eye: [[3, 11], [2, 12], [3, 13]], flame: [[4, 17], [3, 18], [5, 18], [4, 18], [4, 19]], signs: [[-1, 4], [-2, 9], [0, 0]] },
    error: { eye: [[3, 12]], quiet: true, smoke: [[0, 17], [-1, 18], [-2, 17], [-3, 18]] },
    sleep: { eye: [[3, 11], [3, 12]], quiet: true, signs: [[-1, 17], [-1, 18], [0, 17], [1, 17], [1, 18], [-3, 19], [-2, 19]] },
  };
  const m = $derived(MOODS[mood] ?? MOODS.idle);
</script>

<!-- The grid is shifted by (1, 4) so signs above the head and the flame stay inside the view box. -->
<svg class="vouivre {mood}" viewBox="0 0 22 20" shape-rendering="crispEdges" aria-hidden="true">
  <g transform="translate(1 4)">
    <ellipse class="halo" cx="8" cy="12.5" rx="8" ry="3" />
    <g class="sprite">
      {#each Object.entries(LAYERS) as [name, px] (name)}
        <g class="l-{name}">
          {#each px as p}
            {#if !(p.k === 'tongue' && m.quiet)}<rect x={p.x} y={p.y} width="1.02" height="1.02" class={p.k} />{/if}
          {/each}
          {#if name === 'head'}
            {#each m.eye as [y, x]}<rect {x} {y} width="1.02" height="1.02" class="eye" />{/each}
          {/if}
        </g>
      {/each}
      {#if m.flame}
        <g class="l-flame">{#each m.flame as [y, x], i}<rect {x} {y} width="1" height="1" class={i === 3 ? 'core' : 'tongue'} />{/each}</g>
      {/if}
      {#if m.smoke}
        <g class="l-smoke">{#each m.smoke as [y, x], i}<rect {x} {y} width="1" height="1" class="smoke" style="opacity: {0.75 - i * 0.12}" />{/each}</g>
      {/if}
      {#if m.signs}
        <g class="l-signs">{#each m.signs as [y, x]}<rect {x} {y} width="1" height="1" class="sign" />{/each}</g>
      {/if}
    </g>
  </g>
</svg>

<style>
  .vouivre {
    --body: var(--accent);
    width: 44px;
    height: 40px;
    flex: none;
    overflow: visible;
  }
  .error {
    --body: #e5484d;
  }
  .scale {
    fill: var(--body);
  }
  .shade {
    fill: color-mix(in srgb, var(--body) 72%, var(--bg));
  }
  .belly {
    fill: color-mix(in srgb, var(--body) 60%, var(--fg));
  }
  .horn {
    fill: color-mix(in srgb, var(--fg) 92%, transparent);
  }
  .wing,
  .tongue {
    fill: var(--tint);
  }
  .vein {
    fill: color-mix(in srgb, var(--tint) 65%, var(--bg));
  }
  .error .wing {
    fill: #8a1c22;
  }
  .error .vein {
    fill: #5e1016;
  }
  .core {
    fill: #fff1c2;
  }
  .eye {
    fill: var(--bg);
  }
  .sign {
    fill: color-mix(in srgb, var(--accent) 85%, transparent);
  }
  .smoke {
    fill: #8b8584;
  }
  .halo {
    fill: var(--body);
    opacity: 0.24;
    filter: blur(1.6px);
  }
  .lost .halo,
  .sleep .halo {
    opacity: 0.1;
  }

  /* Pivots in view box units (grid + 1, 4): neck, shoulder, base of the tail. */
  .sprite,
  g[class^='l-'] {
    transform-box: view-box;
  }
  .l-head {
    transform-origin: 10px 10px;
  }
  .l-wing {
    transform-origin: 8px 11px;
  }
  .l-tail {
    transform-origin: 12px 17px;
  }
  .l-flame {
    transform-origin: 18px 8px;
  }

  /* Poses */
  .point .l-head {
    transform: rotate(10deg);
  }
  .lost .l-head {
    transform: rotate(-8deg);
  }
  .lost .l-wing,
  .sleep .l-wing {
    transform: rotate(-12deg);
  }

  /* Motion. Durations follow the preset's animation_ms (--anim, 120 ms by default). */
  .idle .sprite {
    animation: breathe calc(var(--anim) * 25) ease-in-out infinite;
  }
  .idle .l-wing {
    animation: flap calc(var(--anim) * 12.5) ease-in-out infinite;
  }
  .idle .l-tail {
    animation: wag calc(var(--anim) * 17) ease-in-out infinite;
  }
  .idle .halo {
    animation: glow calc(var(--anim) * 25) ease-in-out infinite;
  }
  .idle .eye {
    transform-box: fill-box;
    transform-origin: center;
    animation: blink calc(var(--anim) * 38) step-end infinite;
  }
  .listen .l-head {
    animation: nod calc(var(--anim) * 5) ease-out infinite;
  }
  .listen .l-wing {
    animation: flap calc(var(--anim) * 4) ease-in-out infinite;
  }
  .listen .l-tail {
    animation: wag calc(var(--anim) * 8) ease-in-out infinite;
  }
  .listen .l-signs {
    animation: pulse calc(var(--anim) * 5) ease-in-out infinite;
  }
  .point .sprite {
    animation: hop-down calc(var(--anim) * 3) ease-out;
  }
  .lost .l-signs {
    animation: float calc(var(--anim) * 16) ease-in-out infinite;
  }
  .joy .sprite {
    animation: jump calc(var(--anim) * 5) cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  .joy .l-flame {
    animation: puff calc(var(--anim) * 6) ease-out both;
  }
  .joy .l-wing {
    animation: flap calc(var(--anim) * 3) ease-in-out infinite;
  }
  .error .sprite {
    animation: shake calc(var(--anim) * 3) linear;
  }
  .error .l-smoke,
  .sleep .l-signs {
    animation: rise calc(var(--anim) * 14) ease-out infinite;
  }
  .sleep .sprite {
    animation: breathe calc(var(--anim) * 50) ease-in-out infinite;
  }
  .arrive .sprite {
    animation: arrive calc(var(--anim) * 2) ease-out both;
  }
  .arrive .l-wing {
    transform: rotate(25deg);
    transition: transform calc(var(--anim) * 3) ease-out;
  }

  @keyframes breathe {
    50% {
      transform: translateY(-0.4px);
    }
  }
  @keyframes flap {
    50% {
      transform: rotate(-12deg);
    }
  }
  @keyframes wag {
    50% {
      transform: rotate(10deg);
    }
  }
  @keyframes glow {
    50% {
      opacity: 0.4;
    }
  }
  @keyframes blink {
    94% {
      transform: scaleY(0.15);
    }
  }
  @keyframes nod {
    20% {
      transform: rotate(-6deg);
    }
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  @keyframes hop-down {
    30% {
      transform: translateY(0.4px);
    }
  }
  @keyframes float {
    50% {
      transform: translateY(-0.5px);
    }
  }
  @keyframes jump {
    50% {
      transform: translateY(-1.8px);
    }
  }
  @keyframes puff {
    from {
      opacity: 0;
      transform: scale(0.2);
    }
    40% {
      opacity: 1;
      transform: scale(1.2);
    }
    to {
      opacity: 0;
      transform: scale(0.2);
    }
  }
  @keyframes shake {
    20%,
    60% {
      transform: translateX(-0.4px);
    }
    40%,
    80% {
      transform: translateX(0.4px);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(0.5px);
    }
    30% {
      opacity: 1;
    }
    to {
      opacity: 0;
      transform: translateY(-1.5px);
    }
  }
  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateX(-3.5px);
    }
  }

  :global(:root[data-still]) .vouivre *,
  :global(:root[data-still]) .vouivre {
    animation: none !important;
    transition: none !important;
  }
  @media (prefers-reduced-motion: reduce) {
    .vouivre,
    .vouivre * {
      animation: none !important;
      transition: none !important;
    }
  }
</style>
