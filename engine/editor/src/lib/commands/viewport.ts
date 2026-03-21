import { registerCommandHandler } from '../dispatch';
import { commands } from '../bindings';

export function registerViewportHandlers(): void {
  // viewport.screenshot is in RUST_HANDLED — run_command routes it to Rust directly.
  // We register a TypeScript handler that forwards via the typed run_command binding
  // so callers can use dispatchCommand('viewport.screenshot') uniformly.
  registerCommandHandler('viewport.screenshot', async () => {
    await commands.runCommand('viewport.screenshot', null);
  });
  registerCommandHandler('viewport.toggle_grid', async () => {
    console.warn('viewport.toggle_grid: not yet implemented');
  });
  registerCommandHandler('viewport.toggle_gizmos', async () => {
    console.warn('viewport.toggle_gizmos: not yet implemented');
  });
}
