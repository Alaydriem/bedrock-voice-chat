/**
 * Radial — the BVC design system.
 *
 * Three layers, and you only ever need one of them at a time:
 *
 *  - **CSS classes** (`rad-*`) for everything that is not a canvas. Import
 *    `css/radial.css` and write plain markup.
 *  - **Bindings** for the four generated visuals: the mark, the ring, a level meter, a
 *    server glyph. Either mount them from `data-rad-*` attributes with `Mount.scan`, or
 *    construct them directly and hand them a data source.
 *  - **Svelte components** wrapping both, for app code.
 *
 * Nothing here reaches for Tauri, SvelteKit or the network. The reference pages under
 * `examples/` are the same code with synthetic data.
 */

// ---- the mark, and everything drawn from it ----
export { MarkData, type MarkColumn } from "./core/mark/MarkData";
export { MarkRenderer, type MarkPaint } from "./core/mark/MarkRenderer";
export { RingGeometry } from "./core/ring/RingGeometry";
export { RingRenderer, type RingPaint } from "./core/ring/RingRenderer";
export type { RingSource } from "./core/ring/RingSource";
export { ScopeBuffer } from "./core/ring/ScopeBuffer";
export { ScopeRenderer, type ScopePaint } from "./core/ring/ScopeRenderer";
export { TimelineRenderer, type TimelineLane, type TimelinePaint } from "./core/timeline/TimelineRenderer";
export { TimelineEnvelope } from "./core/timeline/TimelineEnvelope";
export { ServerGlyph, type Glyph } from "./core/glyph/ServerGlyph";

// ---- canvas plumbing ----
export { AnimationLoop, type Frame } from "./core/canvas/AnimationLoop";
export { Surface } from "./core/canvas/Surface";
export { Visibility } from "./core/canvas/Visibility";

// ---- maths and colour ----
export { Color, type Channels } from "./core/color/Color";
export { Ease } from "./core/math/Ease";
export { Hash } from "./core/math/Hash";

// ---- the boot sequence, and the loader it becomes ----
export { IntroSequence } from "./core/intro/IntroSequence";
export { INTRO_DEFAULTS, BRAND_LIFT, type IntroConfig, type IntroEndState } from "./core/intro/IntroConfig";
export { INTRO_PHASES, IntroMarks, type IntroPhase } from "./core/intro/IntroPhases";
export { Loader, type LoaderOptions } from "./core/intro/Loader";
export { LoaderStatus, type LoaderStatusFrame, type LoaderStatusOptions } from "./core/intro/LoaderStatus";
export { CanvasRecorder } from "./core/intro/CanvasRecorder";

// ---- where data comes from ----
export {
  type LevelSource,
  type LevelListener,
  type Unsubscribe,
  PushLevelSource,
  ConstantLevelSource,
} from "./core/sources/LevelSource";
export { SyntheticLevelSource, type SyntheticOptions } from "./core/sources/SyntheticLevelSource";
export { PlayerHue } from "./core/sources/PlayerHue";
export { PositionalSource, type Placement } from "./core/sources/PositionalSource";

// ---- icons ----
export { Icons, RAD_ICONS, type IconName } from "./core/icons/Icons";

// ---- element bindings ----
export type { Binding } from "./bindings/Binding";
export { Mount } from "./bindings/Mount";
export { MarkBinding, type MarkOptions } from "./bindings/MarkBinding";
export { LevelMeterBinding, type LevelMeterOptions } from "./bindings/LevelMeterBinding";
export { RingBinding, type RingMode, type RingOptions } from "./bindings/RingBinding";
export { ScopeBinding, type ScopeOptions } from "./bindings/ScopeBinding";
export { GlyphBinding, type GlyphOptions } from "./bindings/GlyphBinding";
export { TimelineBinding, type TimelineOptions } from "./bindings/TimelineBinding";
export { IconBinding } from "./bindings/IconBinding";

// ---- behaviour ----
export { Toast } from "./core/controllers/Toast";
export { Menu, MENU_DIVIDER, type MenuItem, type MenuEntry } from "./core/controllers/Menu";
export { Modal } from "./core/controllers/Modal";
export { Sheet } from "./core/controllers/Sheet";
export { FormControls, type FormControlHooks } from "./core/controllers/FormControls";
export { SelectControl } from "./core/controllers/SelectControl";
export { KeybindCapture } from "./core/controllers/KeybindCapture";
export { DragReorder } from "./core/controllers/DragReorder";
export { TableController, type TableOptions, type TableView, type TableColumn } from "./core/controllers/TableController";
export { TypedConfirm } from "./core/controllers/TypedConfirm";
export { Conditional } from "./core/controllers/Conditional";
export { Handoff, type Point } from "./core/controllers/Handoff";
export { SelfState, type SelfSnapshot, type VoiceMode } from "./core/controllers/SelfState";
export { Diagnostics, type DiagnosticsInput, type KvGroup, type Severity } from "./core/controllers/Diagnostics";
export { KvGridView } from "./core/controllers/KvGridView";
export { ChatLog, type ChatMessage, type ChatOptions } from "./core/controllers/ChatLog";
export { LogConsole, type LogLevel, type LogLine } from "./core/controllers/LogConsole";
export { StepFlow, type StepFlowOptions } from "./core/controllers/StepFlow";
export { ScreenRouter } from "./core/controllers/ScreenRouter";
