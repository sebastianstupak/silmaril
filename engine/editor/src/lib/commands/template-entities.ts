import { registerCommandHandler } from '../dispatch';

// Template entity commands — stubs until the scene mutation system is implemented.

export function registerTemplateEntityHandlers(): void {
  for (const id of [
    'template.new_entity',
    'template.delete_entity',
    'template.duplicate_entity',
    'template.focus_entity',
  ] as const) {
    registerCommandHandler(id, async () => {
      console.warn(`${id}: template system not yet implemented`);
    });
  }
}
