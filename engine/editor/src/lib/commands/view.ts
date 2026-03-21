import { registerCommandHandler } from '../dispatch';
import {
  toggleHierarchy,
  toggleInspector,
  toggleConsole,
  toggleAssetBrowser,
} from '../stores/layout';

export function registerViewHandlers(): void {
  registerCommandHandler('view.toggle_hierarchy', async () => { toggleHierarchy(); });
  registerCommandHandler('view.toggle_inspector', async () => { toggleInspector(); });
  registerCommandHandler('view.toggle_console', async () => { toggleConsole(); });
  registerCommandHandler('view.toggle_asset_browser', async () => { toggleAssetBrowser(); });
  // Zoom commands operate on the viewport canvas; not yet wired to a store.
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
