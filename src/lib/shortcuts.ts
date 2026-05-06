type Handler = () => void;

export interface ShortcutHandlers {
  newNote: Handler;
  togglePreview: Handler;
  openSettings: Handler;
}

export function attachShortcuts(handlers: ShortcutHandlers): () => void {
  const onKeyDown = (e: KeyboardEvent) => {
    if (!e.metaKey || e.ctrlKey || e.altKey) return;
    const key = e.key.toLowerCase();
    if (key === 'n') {
      e.preventDefault();
      handlers.newNote();
    } else if (key === 'e') {
      e.preventDefault();
      handlers.togglePreview();
    } else if (e.key === ',') {
      e.preventDefault();
      handlers.openSettings();
    }
  };
  window.addEventListener('keydown', onKeyDown);
  return () => window.removeEventListener('keydown', onKeyDown);
}
