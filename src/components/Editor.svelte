<script lang="ts">
  import { activeBody, editorScrollTop } from '$lib/stores';
  import { onMount, tick } from 'svelte';

  export let onInput: () => void = () => {};

  let textarea: HTMLTextAreaElement;

  export function focus() {
    textarea?.focus();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key !== 'Tab' || e.metaKey || e.ctrlKey || e.altKey) return;
    e.preventDefault();
    const ta = e.currentTarget as HTMLTextAreaElement;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const value = ta.value;
    const next = value.slice(0, start) + '  ' + value.slice(end);
    ta.value = next;
    ta.selectionStart = ta.selectionEnd = start + 2;
    activeBody.set(next);
    onInput();
  }

  function handleScroll(e: Event) {
    editorScrollTop.set((e.currentTarget as HTMLTextAreaElement).scrollTop);
  }

  onMount(async () => {
    await tick();
    if (textarea) textarea.scrollTop = $editorScrollTop;
  });
</script>

<textarea
  bind:this={textarea}
  bind:value={$activeBody}
  on:input={onInput}
  on:keydown={handleKeyDown}
  on:scroll={handleScroll}
  spellcheck="true"
  autocomplete="off"
  autocapitalize="off"
  {...{ autocorrect: 'off' }}
></textarea>

<style>
  textarea {
    flex: 1;
    width: 100%;
    border: none;
    outline: none;
    resize: none;
    background: var(--bg);
    color: var(--fg);
    caret-color: var(--fg);
    padding: 32px;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 14px;
    line-height: 1.6;
  }
</style>
