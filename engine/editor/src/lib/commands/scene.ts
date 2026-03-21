import { registerCommandHandler } from '../dispatch';

// Scene commands are stubs — the scene mutation system is not yet implemented.

export function registerSceneHandlers(): void {
  for (const id of [
    'scene.new_entity',
    'scene.delete_entity',
    'scene.duplicate_entity',
    'scene.focus_entity',
  ] as const) {
    registerCommandHandler(id, async () => {
      console.warn(`${id}: scene system not yet implemented`);
    });
  }
}
