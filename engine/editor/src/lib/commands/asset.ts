import { registerCommandHandler } from '../dispatch';
import { commands } from '../bindings';

export function registerAssetHandlers(): void {
  // asset.scan delegates to the run_command IPC, which will eventually route
  // to the Rust scan_assets handler. args must include the project path;
  // the caller is expected to pass it via dispatchCommand args.
  registerCommandHandler('asset.scan', async (args) => {
    await commands.runCommand('asset.scan', (args as object | null | undefined) ?? null);
  });
  registerCommandHandler('asset.import', async () => {
    console.warn('asset.import: not yet implemented');
  });
  registerCommandHandler('asset.refresh', async () => {
    console.warn('asset.refresh: not yet implemented');
  });
}
