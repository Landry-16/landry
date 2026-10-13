<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let query = $state('');
  let input;

  $effect(() => {
    const un = listen('landry://shown', () => {
      input?.focus();
      input?.select();
      // Two frames: the first is queued with the show, the second proves it was painted.
      requestAnimationFrame(() => requestAnimationFrame(() => invoke('painted')));
    });
    return () => un.then((f) => f());
  });

  function onkeydown(e) {
    if (e.key === 'Escape') {
      query = '';
      invoke('hide');
    }
  }
</script>

<main class="bar">
  <input
    bind:this={input}
    bind:value={query}
    {onkeydown}
    placeholder="Tape pour chercher"
    aria-label="Recherche Landry"
    spellcheck="false"
    autocomplete="off"
  />
</main>

<style>
  .bar {
    background: var(--bar-bg);
    color: var(--bar-fg);
    border-radius: var(--bar-radius);
    padding: 12px 16px;
    margin: 8px;
  }
  input {
    width: 100%;
    background: transparent;
    border: 0;
    outline: none;
    color: inherit;
    font: inherit;
    font-size: 1.3em;
    caret-color: var(--bar-accent);
  }
  input::placeholder {
    color: var(--bar-muted);
  }
</style>
