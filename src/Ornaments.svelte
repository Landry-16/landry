<script>
  // Decoration around the bar, drawn from the Figma presets. Pure SVG, colours from the preset
  // (--accent, --tint). The back layer sits behind the bar, the front layer overlaps its edges.
  let { ornament = 'none' } = $props();

  const SCROLL = 'M 0 30 C 10 30 18 22 18 12 C 18 4 10 0 4 4 C 0 8 4 14 9 11 M 18 12 C 30 26 52 30 70 26 C 80 24 86 18 84 10 C 82 4 74 4 74 10';
  const FILIGREE = 'M 40 2 C 10 8 0 32 14 52 C 24 66 44 60 40 46 C 37 36 26 40 30 48 M 14 52 C 4 72 10 98 34 102 C 46 104 52 92 44 86';
  const SWIRL = 'M 2 40 C 12 10 52 0 62 22 C 68 36 52 44 44 36 C 38 30 44 22 52 26';
  const CLOUD = 'M 8 44 C -4 44 -4 28 8 28 C 6 14 24 8 32 18 C 36 4 58 2 64 16 C 74 8 92 14 88 30 C 100 30 102 44 90 44 Z';
  const RAYS = Array.from({ length: 24 }, (_, i) => {
    const a = (i / 24) * Math.PI * 2, r2 = i % 2 ? 78 : 92;
    return `M ${(90 + Math.cos(a) * 64).toFixed(1)} ${(90 + Math.sin(a) * 64).toFixed(1)} L ${(90 + Math.cos(a) * r2).toFixed(1)} ${(90 + Math.sin(a) * r2).toFixed(1)}`;
  }).join(' ');
</script>

{#if ornament !== 'none'}
  <div class="orn back" aria-hidden="true">
    {#if ornament === 'fresco'}
      <svg class="sun" viewBox="0 0 180 180">
        <path d={RAYS} class="line" stroke-width="2" />
        <circle cx="90" cy="90" r="58" class="tint" />
      </svg>
      <svg class="clouds" viewBox="0 0 200 80">
        <path d={CLOUD} class="ink" transform="translate(0 6) scale(1.4)" />
        <path d={CLOUD} class="ink" transform="translate(110 30)" />
        <path d="M 160 70 C 166 54 182 54 184 64 C 185 72 176 74 174 68" class="line" stroke-width="1.5" />
      </svg>
    {/if}
  </div>

  <div class="orn front" aria-hidden="true">
    {#if ornament === 'baroque'}
      <svg class="crest" viewBox="0 0 220 64">
        <defs>
          <radialGradient id="orn-medal">
            <stop offset="0" stop-color="#fff4e6" />
            <stop offset="0.45" stop-color="var(--tint)" />
            <stop offset="1" stop-color="#2a0509" />
          </radialGradient>
        </defs>
        <path d={SCROLL} class="line" stroke-width="2" transform="translate(16 14)" />
        <path d={SCROLL} class="line" stroke-width="2" transform="translate(204 14) scale(-1 1)" />
        <path d="M 110 2 C 116 10 116 18 110 26 C 104 18 104 10 110 2 Z" class="gold" />
        <circle cx="110" cy="42" r="15" fill="url(#orn-medal)" class="ring" stroke-width="2.5" />
      </svg>
      {#each ['left', 'right'] as side}
        <svg class="rose {side}" viewBox="-24 -24 48 48">
          <ellipse cx="-16" cy="12" rx="11" ry="4" transform="rotate(30 -16 12)" class="leaf" />
          {#each [[0, -10], [9, -3], [6, 8], [-6, 8], [-9, -3]] as [x, y]}<ellipse cx={x} cy={y} rx="9" ry="8" class="petal" />{/each}
          <circle r="9" class="heart" />
          <path d="M 0 0 C 4 -4 8 0 4 4 C 0 8 -8 4 -6 -2 C -4 -9 6 -10 9 -3" class="spiral" />
        </svg>
        <svg class="filigree {side}" viewBox="0 0 52 106"><path d={FILIGREE} class="line" stroke-width="2" /></svg>
      {/each}
      <svg class="divider" viewBox="0 0 300 16" preserveAspectRatio="none">
        <path d="M 0 8 L 112 8 M 188 8 L 300 8" class="line faint" stroke-width="1" vector-effect="non-scaling-stroke" />
        <path d="M 116 8 C 126 2 136 2 146 8 C 136 14 126 14 116 8 M 154 8 C 164 2 174 2 184 8 C 174 14 164 14 154 8" class="line" stroke-width="1" vector-effect="non-scaling-stroke" />
        <rect x="146" y="4" width="8" height="8" transform="rotate(45 150 8)" class="gold" />
      </svg>
    {:else if ornament === 'silk'}
      <svg class="pearl" viewBox="0 0 100 40">
        <defs>
          <radialGradient id="orn-pearl" cx="0.4" cy="0.35">
            <stop offset="0" stop-color="#fff6f0" />
            <stop offset="0.55" stop-color="var(--tint)" />
            <stop offset="1" stop-color="#7a2c44" />
          </radialGradient>
        </defs>
        <path d="M 4 30 C 24 6 76 6 96 30" class="line" stroke-width="2" />
        <circle cx="50" cy="22" r="16" fill="url(#orn-pearl)" class="ring" stroke-width="2" />
      </svg>
      <svg class="swirl tl" viewBox="0 0 70 46"><path d={SWIRL} class="line" stroke-width="2.5" /></svg>
      <svg class="swirl br" viewBox="0 0 70 46"><path d={SWIRL} class="line" stroke-width="2.5" /></svg>
      <svg class="ribbon" viewBox="0 0 64 48"><path d="M 2 2 C 22 12 12 32 36 42 C 52 48 62 32 50 26" class="line tint-line" stroke-width="2" /></svg>
    {:else if ornament === 'pixel'}
      {#each ['tl', 'tr', 'bl', 'br'] as corner}
        <svg class="corner {corner}" viewBox="0 0 24 24" shape-rendering="crispEdges">
          <path d="M0 0h24v8H8v16H0z" class="gold" />
        </svg>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .orn {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .back {
    z-index: 0;
  }
  .front {
    z-index: 2;
  }
  svg {
    position: absolute;
    overflow: visible;
  }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-linecap: round;
  }
  .faint {
    opacity: 0.5;
  }
  .tint-line {
    stroke: var(--tint);
  }
  .gold {
    fill: var(--accent);
  }
  .tint {
    fill: var(--tint);
  }
  .ring {
    stroke: var(--accent);
  }
  .ink {
    fill: color-mix(in srgb, var(--bg) 60%, black);
    stroke: color-mix(in srgb, var(--accent) 60%, transparent);
    stroke-width: 1.5;
  }
  .petal {
    fill: color-mix(in srgb, var(--tint) 78%, black);
    stroke: color-mix(in srgb, var(--tint) 45%, black);
  }
  .heart {
    fill: var(--tint);
  }
  .spiral {
    fill: none;
    stroke: color-mix(in srgb, var(--tint) 45%, black);
    stroke-width: 1.4;
  }
  .leaf {
    fill: color-mix(in srgb, var(--accent) 55%, transparent);
  }

  .crest {
    width: 220px;
    height: 64px;
    left: 50%;
    top: -50px;
    transform: translateX(-50%);
  }
  .rose {
    width: 48px;
    height: 48px;
    top: -20px;
  }
  .rose.left {
    left: -20px;
  }
  .rose.right {
    right: -20px;
  }
  .filigree {
    width: 34px;
    height: 70px;
    top: 56px;
  }
  .filigree.left {
    left: -30px;
  }
  .filigree.right {
    right: -30px;
    transform: scaleX(-1);
  }
  .divider {
    width: 60%;
    height: 16px;
    left: 20%;
    bottom: -30px;
  }
  .pearl {
    width: 100px;
    height: 40px;
    left: 50%;
    top: -26px;
    transform: translateX(-50%);
  }
  .swirl {
    width: 62px;
    height: 40px;
  }
  .swirl.tl {
    left: -36px;
    top: -26px;
  }
  .swirl.br {
    right: -36px;
    bottom: -26px;
    transform: rotate(180deg);
  }
  .ribbon {
    width: 48px;
    height: 36px;
    right: 18px;
    top: -30px;
  }
  .sun {
    width: 180px;
    height: 180px;
    right: -96px;
    top: -104px;
  }
  .clouds {
    width: 200px;
    height: 80px;
    left: -96px;
    bottom: -52px;
  }
  .corner {
    width: 24px;
    height: 24px;
  }
  .corner.tl {
    left: -10px;
    top: -10px;
  }
  .corner.tr {
    right: -10px;
    top: -10px;
    transform: scaleX(-1);
  }
  .corner.bl {
    left: -10px;
    bottom: -10px;
    transform: scaleY(-1);
  }
  .corner.br {
    right: -10px;
    bottom: -10px;
    transform: scale(-1);
  }

  /* Ornaments arrive just after the bar. */
  .front > svg,
  .back > svg {
    animation: settle calc(var(--anim) * 2.5) ease-out both;
    animation-delay: calc(var(--anim) / 2);
  }
  @keyframes settle {
    from {
      opacity: 0;
    }
  }
  :global(:root[data-still]) .orn svg {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .orn svg {
      animation: none;
    }
  }
  @media (forced-colors: active) {
    .orn {
      display: none;
    }
  }
</style>
