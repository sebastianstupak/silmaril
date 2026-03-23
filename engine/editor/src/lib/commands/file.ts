import { registerCommandHandler } from '../dispatch';
import { commands } from '../bindings';

export function registerFileHandlers(): void {
  registerCommandHandler('file.save_template', async () => {
    await commands.runCommand('file.save_scene', null);
  });
  registerCommandHandler('file.save_template_as', async () => {
    await commands.runCommand('file.save_scene_as', null);
  });
  registerCommandHandler('file.open_template', async () => {
    await commands.runCommand('file.open_scene', null);
  });
  // file.new_project and file.open_project are UI-driven (open dialogs) —
  // they are not yet wired on the Rust side, so we stub them here.
  registerCommandHandler('file.new_project', async () => {
    console.warn('file.new_project: not yet implemented');
  });
  registerCommandHandler('file.open_project', async () => {
    console.warn('file.open_project: not yet implemented');
  });
}
