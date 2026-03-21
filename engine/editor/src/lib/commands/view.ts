import { registerCommandHandler } from '../dispatch';

// View commands toggle UI panels. Panel layout state currently lives in
// App.svelte and has not yet been extracted to a standalone store (Task 10).
// All handlers are stubs until Task 10 creates the layout store.

export function registerViewHandlers(): void {
  registerCommandHandler('view.toggle_hierarchy', async () => {
    console.warn('view.toggle_hierarchy: layout store not yet extracted (Task 10)');
  });
  registerCommandHandler('view.toggle_inspector', async () => {
    console.warn('view.toggle_inspector: layout store not yet extracted (Task 10)');
  });
  registerCommandHandler('view.toggle_console', async () => {
    console.warn('view.toggle_console: layout store not yet extracted (Task 10)');
  });
  registerCommandHandler('view.toggle_asset_browser', async () => {
    console.warn('view.toggle_asset_browser: layout store not yet extracted (Task 10)');
  });
  registerCommandHandler('view.zoom_in', async () => {
    console.warn('view.zoom_in: not yet implemented');
  });
  registerCommandHandler('view.zoom_out', async () => {
    console.warn('view.zoom_out: not yet implemented');
  });
  registerCommandHandler('view.zoom_reset', async () => {
    console.warn('view.zoom_reset: not yet implemented');
  });
}
