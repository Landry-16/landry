<script>
  // Slow particles behind the results: fireflies by default, petals, gold leaves or pixels
  // depending on the ornament. CSS only, positions spread deterministically.
  let { count = 0, ornament = 'none' } = $props();
  const items = $derived(
    Array.from({ length: count }, (_, i) => ({
      left: (i * 37.3 + 11) % 100,
      top: (i * 53.7 + 7) % 100,
      dur: 9 + ((i * 7) % 8),
      delay: -((i * 2.3) % 14),
      size: 0.8 + ((i * 3) % 4) / 4,
      sway: ((i % 2 ? 1 : -1) * (12 + ((i * 5) % 20))).toFixed(0),
    })),
  );
</script>

{#if count > 0}
  <div class="particles {ornament}" aria-hidden="true">
    {#each items as p}
      <span style="left: {p.left}%; top: {p.top}%; --s: {p.size}; --sway: {p.sway}px; animation-duration: {p.dur}s; animation-delay: {p.delay}s"></span>
    {/each}
  </div>
{/if}

<style>
  .particles {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 0;
  }
  span {
    position: absolute;
    width: calc(var(--s) * 4px);
    height: calc(var(--s) * 4px);
    border-radius: 50%;
    background: var(--accent);
    opacity: 0;
    animation: hover linear infinite;
  }
  /* Fireflies hover and blink; the others fall and sway. */
  .none span {
    box-shadow: 0 0 6px var(--accent);
  }
  .silk span,
  .baroque span {
    width: calc(var(--s) * 9px);
    height: calc(var(--s) * 5px);
    border-radius: 60% 0;
    background: var(--tint);
    animation-name: fall;
  }
  .fresco span {
    width: calc(var(--s) * 10px);
    height: calc(var(--s) * 6px);
    border-radius: 50%;
    background: radial-gradient(circle, #3fb8a8 0 22%, var(--accent) 26%);
    animation-name: fall;
  }
  .pixel span {
    border-radius: 0;
    background: var(--tint);
    animation-name: twinkle;
    animation-timing-function: steps(4);
  }
  @keyframes hover {
    0%,
    100% {
      opacity: 0;
      transform: translate(0, 0);
    }
    30%,
    70% {
      opacity: 0.55;
    }
    50% {
      transform: translate(var(--sway), -10px);
    }
  }
  @keyframes fall {
    from {
      opacity: 0;
      transform: translate(0, -60px) rotate(0);
    }
    15%,
    85% {
      opacity: 0.5;
    }
    to {
      opacity: 0;
      transform: translate(var(--sway), 90px) rotate(240deg);
    }
  }
  @keyframes twinkle {
    0%,
    100% {
      opacity: 0;
    }
    50% {
      opacity: 0.5;
    }
  }
  :global(:root[data-still]) .particles {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .particles {
      display: none;
    }
  }
</style>
