import { registerFileHandlers } from './file';
import { registerEditHandlers } from './edit';
import { registerViewHandlers } from './view';
import { registerTemplateEntityHandlers } from './template-entities';
import { registerAssetHandlers } from './asset';
import { registerBuildHandlers } from './build';
import { registerViewportHandlers } from './viewport';
import { registerTemplateHandlers } from './template';

export function registerAllHandlers(): void {
  registerFileHandlers();
  registerEditHandlers();
  registerViewHandlers();
  registerTemplateEntityHandlers();
  registerAssetHandlers();
  registerBuildHandlers();
  registerViewportHandlers();
  registerTemplateHandlers();
}
