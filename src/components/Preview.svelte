<script lang="ts">
  import { activeBody } from '$lib/stores';
  import { renderMarkdown } from '$lib/markdown';
  import { open as shellOpen } from '@tauri-apps/plugin-shell';

  let html = $derived(renderMarkdown($activeBody));

  function handleClick(e: MouseEvent) {
    const target = (e.target as HTMLElement | null)?.closest('a');
    if (!target) return;
    const href = target.getAttribute('data-external') ?? target.getAttribute('href');
    if (!href) return;
    e.preventDefault();
    shellOpen(href).catch(() => {});
  }
</script>

<div class="preview" onclick={handleClick} role="presentation">
  {@html html}
</div>

<style>
  .preview {
    flex: 1;
    width: 100%;
    overflow-y: auto;
    background: var(--bg);
    color: var(--fg);
    padding: 32px;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 14px;
    line-height: 1.6;
  }
  .preview :global(h1) { font-size: 1.7em; margin: 0.6em 0 0.4em; font-weight: 700; }
  .preview :global(h2) { font-size: 1.45em; margin: 0.6em 0 0.4em; font-weight: 700; }
  .preview :global(h3) { font-size: 1.25em; margin: 0.6em 0 0.4em; font-weight: 700; }
  .preview :global(h4) { font-size: 1.1em; margin: 0.6em 0 0.4em; font-weight: 700; }
  .preview :global(h5) { font-size: 1em; margin: 0.6em 0 0.4em; font-weight: 700; }
  .preview :global(h6) { font-size: 0.9em; margin: 0.6em 0 0.4em; font-weight: 700; color: var(--muted); }
  .preview :global(p) { margin: 0.4em 0 0.8em; }
  .preview :global(ul), .preview :global(ol) { padding-left: 1.4em; margin: 0.4em 0 0.8em; }
  .preview :global(li) { margin: 0.15em 0; }
  .preview :global(a) { color: var(--accent); text-decoration: underline; }
  .preview :global(code) {
    font-family: inherit;
    background: var(--code-bg);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 0.95em;
  }
  .preview :global(pre) {
    background: var(--code-bg);
    padding: 10px 12px;
    border-radius: 4px;
    overflow-x: auto;
    margin: 0.6em 0;
  }
  .preview :global(pre code) {
    background: none;
    padding: 0;
  }
  .preview :global(blockquote) {
    border-left: 3px solid var(--quote-border);
    padding-left: 12px;
    margin: 0.6em 0;
    color: var(--muted);
  }
  .preview :global(table) {
    border-collapse: collapse;
    margin: 0.6em 0;
  }
  .preview :global(th), .preview :global(td) {
    border: 1px solid var(--bar-border);
    padding: 4px 8px;
    text-align: left;
  }
  .preview :global(hr) {
    border: none;
    border-top: 1px solid var(--bar-border);
    margin: 1em 0;
  }
  .preview :global(input[type="checkbox"]) {
    margin-right: 6px;
  }
  .preview :global(li.task-list-item) {
    list-style: none;
    margin-left: -1.4em;
  }
</style>
