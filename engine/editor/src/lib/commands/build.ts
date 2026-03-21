import { registerCommandHandler } from '../dispatch';

// Build commands are stubs — the build system is not yet implemented.

export function registerBuildHandlers(): void {
  for (const id of ['build.run', 'build.build', 'build.package'] as const) {
    registerCommandHandler(id, async () => {
      console.warn(`${id}: build system not yet implemented`);
    });
  }
}
