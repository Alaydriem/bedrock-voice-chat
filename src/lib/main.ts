/**
 * Client entry point. Importing a painter module registers it with the engine,
 * so this file's import list is the set of visuals the site can draw.
 */
import './canvas/mosaic';
import './canvas/ring';
import './canvas/channels';
import './canvas/matrix';
import './canvas/tracks';
import './canvas/scene';

import { startCanvasEngine } from './canvas/engine';
import { initAudience } from './audience';
import { initReveal } from './reveal';
import { initNav } from './nav';
import { initRotators } from './rotator';

initAudience();
initNav();
initReveal();
initRotators();
startCanvasEngine();
