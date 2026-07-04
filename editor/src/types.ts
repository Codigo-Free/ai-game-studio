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
  | { type: "click" }
  | { type: "scene_start" }
  | { type: "animation_end"; animation: string }
  | { type: "collision"; with?: string };

export type ActionSpec =
  | { type: "move"; dx: number; dy: number }
  | { type: "goto_scene"; scene: string }
  | { type: "play_animation"; animation: string };

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

export interface Components {
  transform2d?: Transform2D;
  sprite?: SpriteComponent;
  camera2d?: Camera2DComponent;
  rigidbody2d?: Rigidbody2DComponent;
  collider2d?: Collider2DComponent;
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
  entities: EntityNode[];
  animations?: SceneAnimation[];
}

export type AssetKind = "image" | "audio" | "font" | "other";

export interface Asset {
  id: string;
  kind: AssetKind;
  path: string;
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
