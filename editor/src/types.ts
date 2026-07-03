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

export interface Components {
  transform2d?: Transform2D;
  sprite?: SpriteComponent;
  camera2d?: Camera2DComponent;
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
