<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { theme } from '$lib/stores';
  import type { Theme } from '$lib/colors';

  function applyTheme(t: Theme) {
    document.documentElement.setAttribute('data-theme', t);
    theme.set(t);
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;

    (async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        const current = await win.theme();
        applyTheme(current === 'dark' ? 'dark' : 'light');
        unlisten = await win.onThemeChanged(({ payload }) => {
          applyTheme(payload === 'dark' ? 'dark' : 'light');
        });
      } catch {
        const mq = window.matchMedia('(prefers-color-scheme: dark)');
        applyTheme(mq.matches ? 'dark' : 'light');
        const handler = (e: MediaQueryListEvent) => applyTheme(e.matches ? 'dark' : 'light');
        mq.addEventListener('change', handler);
        unlisten = () => mq.removeEventListener('change', handler);
      }
    })();

    return () => {
      if (unlisten) unlisten();
    };
  });
</script>

<slot />
