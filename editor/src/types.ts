// TypeScript mirror of the `.aigs` format (see sdk/aigs-format/SPEC.md).
// The reference implementation is the Rust crate `aigs-project`; the backend
// re-validates everything through it before writing to disk.

export interface FormatHeader {
  kind: string;
  version: number;
}

export interface Transform2D {
  x?: number;
  y?: number;
  rotation?: number;
  scale_x?: number;
  scale_y?: number;
}

export interface SpriteComponent {
  asset: string;
  frame?: number;
  width?: number;
  height?: number;
  opacity?: number;
  layer?: number;
}

export interface Camera2DComponent {
  zoom?: number;
}

export type EventSpec =
  | { type: "key_down"; key: string }
  | { type: "key_pressed"; key: string }
  | { type: "key_released"; key: string }
  | { type: "click" }
  | { type: "scene_start" }
  | { type: "animation_end"; animation: string }
  | { type: "collision"; with?: string };

export type ActionSpec =
  | { type: "move"; dx: number; dy: number }
  | { type: "goto_scene"; scene: string }
  | { type: "play_animation"; animation: string }
  | { type: "play_sound"; asset: string; volume?: number }
  | { type: "emit_particles"; count?: number };

/** Code-free rule: when `on` happens, run `do`. */
export interface Behavior {
  on: EventSpec;
  do: ActionSpec;
}

export type BodyType = "dynamic" | "kinematic" | "static";

export interface Rigidbody2DComponent {
  body?: BodyType;
  gravity_scale?: number;
  vx?: number;
  vy?: number;
  fixed_rotation?: boolean;
}

export interface Collider2DComponent {
  shape?: "box" | "circle";
  width?: number;
  height?: number;
  radius?: number;
  sensor?: boolean;
  restitution?: number;
  friction?: number;
}

export interface AnimatorTransition {
  from: string;
  to: string;
  when: EventSpec;
}

export interface AnimatorComponent {
  initial: string;
  states: Record<string, string>;
  transitions?: AnimatorTransition[];
}

export interface ParticlesComponent {
  asset: string;
  rate?: number;
  lifetime?: number;
  speed?: number;
  direction?: number;
  spread?: number;
  gravity?: number;
  start_scale?: number;
  end_scale?: number;
  start_opacity?: number;
  end_opacity?: number;
  layer?: number;
  emitting?: boolean;
}

export interface Components {
  transform2d?: Transform2D;
  sprite?: SpriteComponent;
  camera2d?: Camera2DComponent;
  rigidbody2d?: Rigidbody2DComponent;
  collider2d?: Collider2DComponent;
  animator?: AnimatorComponent;
  particles?: ParticlesComponent;
  script?: { asset: string };
  behaviors?: Behavior[];
  // Plugin components (namespaced keys) must survive round-trips.
  [key: string]: unknown;
}

export interface EntityNode {
  id: string;
  name: string;
  components?: Components;
  children?: EntityNode[];
}

export interface Keyframe {
  frame: number;
  value: number;
  easing?: "linear" | "ease_in" | "ease_out" | "ease_in_out";
}

export interface Track {
  entity: string;
  property: string;
  keyframes: Keyframe[];
}

export interface SceneAnimation {
  name: string;
  fps: number;
  loop?: boolean;
  tracks?: Track[];
}

export interface Scene {
  format: FormatHeader;
  name: string;
  gravity?: { x?: number; y?: number };
  music?: { asset: string; volume?: number; looped?: boolean };
  entities: EntityNode[];
  animations?: SceneAnimation[];
}

export type AssetKind = "image" | "audio" | "font" | "script" | "other";

export interface Asset {
  id: string;
  kind: AssetKind;
  path: string;
  spritesheet?: { frame_width: number; frame_height: number };
}

export interface Project {
  format: FormatHeader;
  name: string;
  description?: string;
  initial_scene: string;
  scenes: string[];
  assets: Asset[];
}

export interface LoadedScene {
  path: string;
  scene: Scene;
}

export interface LoadedProject {
  root: string;
  manifest_path: string;
  project: Project;
  scenes: LoadedScene[];
}

// --- AI Core: write-assisted chat (milestone M19) ---

export interface EntityToAdd {
  parent_id: string | null;
  entity: EntityNode;
}

export interface EntityToUpdate {
  id: string;
  components_patch: Components;
}

export interface ScriptToWrite {
  asset_id: string;
  filename: string;
  content: string;
}

/** A concrete, reviewable change to the current scene proposed by the AI —
 * validated by the backend against the real format types before it ever
 * reaches this side (see `editor/src-tauri/src/ai.rs`). */
export interface ChangeProposal {
  summary: string;
  entities_to_add: EntityToAdd[];
  entities_to_update: EntityToUpdate[];
  entities_to_remove: string[];
  scripts: ScriptToWrite[];
}
