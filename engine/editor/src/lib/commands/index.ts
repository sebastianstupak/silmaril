import { registerFileHandlers } from './file';
import { registerEditHandlers } from './edit';
import { registerViewHandlers } from './view';
import { registerSceneHandlers } from './scene';
import { registerAssetHandlers } from './asset';
import { registerBuildHandlers } from './build';
import { registerViewportHandlers } from './viewport';
import { registerTemplateHandlers } from './template';

export function registerAllHandlers(): void {
  registerFileHandlers();
  registerEditHandlers();
  registerViewHandlers();
  registerSceneHandlers();
  registerAssetHandlers();
  registerBuildHandlers();
  registerViewportHandlers();
  registerTemplateHandlers();
}
