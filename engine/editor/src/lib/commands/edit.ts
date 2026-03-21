import { registerCommandHandler } from '../dispatch';
import { undo, redo } from '../stores/undo-history';

export function registerEditHandlers(): void {
  // undo/redo are handled entirely on the TypeScript side via the undo-history
  // store, which calls template_undo / template_redo Tauri commands internally.
  registerCommandHandler('edit.undo', async () => {
    await undo();
  });
  registerCommandHandler('edit.redo', async () => {
    await redo();
  });
}
