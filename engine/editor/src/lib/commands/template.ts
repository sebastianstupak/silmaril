import { registerCommandHandler } from '../dispatch';
import { commands } from '../bindings';
import { onTemplateMutated } from '../stores/undo-history';

// Template commands are in RUST_HANDLED — run_command on the Rust side dispatches
// them directly to the template_open / template_close / … Tauri handlers.
// Args format mirrors the existing Tauri IPC: { template_path: string } for most,
// plus { command: TemplateCommand } for template.execute.

export function registerTemplateHandlers(): void {
  registerCommandHandler('template.open', async (args) => {
    await commands.runCommand('template.open', (args as object | null | undefined) ?? null);
  });
  registerCommandHandler('template.close', async (args) => {
    await commands.runCommand('template.close', (args as object | null | undefined) ?? null);
  });
  registerCommandHandler('template.execute', async (args) => {
    await commands.runCommand('template.execute', (args as object | null | undefined) ?? null);
    await onTemplateMutated(); // notify undo history: redo stack cleared
  });
  registerCommandHandler('template.undo', async (args) => {
    await commands.runCommand('template.undo', (args as object | null | undefined) ?? null);
  });
  registerCommandHandler('template.redo', async (args) => {
    await commands.runCommand('template.redo', (args as object | null | undefined) ?? null);
  });
  registerCommandHandler('template.history', async (args) => {
    await commands.runCommand('template.history', (args as object | null | undefined) ?? null);
  });
}
