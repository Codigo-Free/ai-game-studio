// TypeScript mirror of aigs-anim's sampling (keep in sync with
// runtime/crates/aigs-anim/src/lib.rs and sdk/aigs-format/SPEC.md).

import { updateEntity, findEntity } from "./document";
import type {
  EntityNode,
  Keyframe,
  Scene,
  SceneAnimation,
} from "./types";

export const ANIMATABLE_PROPERTIES = [
  "transform2d.x",
  "transform2d.y",
  "transform2d.rotation",
  "transform2d.scale_x",
  "transform2d.scale_y",
  "sprite.opacity",
] as const;

export type Easing = NonNullable<Keyframe["easing"]>;

export function applyEasing(easing: Easing | undefined, t: number): number {
  const clamped = Math.min(1, Math.max(0, t));
  switch (easing ?? "linear") {
    case "ease_in":
      return clamped * clamped;
    case "ease_out":
      return clamped * (2 - clamped);
    case "ease_in_out":
      return clamped < 0.5
        ? 2 * clamped * clamped
        : -1 + (4 - 2 * clamped) * clamped;
    default:
      return clamped;
  }
}

/** Samples a sorted track at a (fractional) frame. Mirrors aigs_anim::sample. */
export function sampleTrack(
  keyframes: Keyframe[],
  frame: number,
): number | null {
  if (keyframes.length === 0) return null;
  const first = keyframes[0];
  if (frame <= first.frame) return first.value;
  for (let i = 0; i + 1 < keyframes.length; i += 1) {
    const from = keyframes[i];
    const to = keyframes[i + 1];
    if (frame < to.frame) {
      const span = Math.max(1, to.frame - from.frame);
      const t = (frame - from.frame) / span;
      return from.value + (to.value - from.value) * applyEasing(from.easing, t);
    }
  }
  return keyframes[keyframes.length - 1].value;
}

/** Reads the current (unanimated) value of an animatable property. */
export function propertyValue(node: EntityNode, property: string): number {
  const transform = node.components?.transform2d;
  const sprite = node.components?.sprite;
  switch (property) {
    case "transform2d.x":
      return transform?.x ?? 0;
    case "transform2d.y":
      return transform?.y ?? 0;
    case "transform2d.rotation":
      return transform?.rotation ?? 0;
    case "transform2d.scale_x":
      return transform?.scale_x ?? 1;
    case "transform2d.scale_y":
      return transform?.scale_y ?? 1;
    case "sprite.opacity":
      return sprite?.opacity ?? 1;
    default:
      return 0;
  }
}

function withProperty(
  node: EntityNode,
  property: string,
  value: number,
): EntityNode {
  const components = { ...node.components };
  if (property.startsWith("transform2d.")) {
    const key = property.slice("transform2d.".length);
    components.transform2d = { ...components.transform2d, [key]: value };
  } else if (property === "sprite.opacity" && components.sprite) {
    components.sprite = { ...components.sprite, opacity: value };
  }
  return { ...node, components };
}

/** Scene with `animation` evaluated at `frame` (viewport preview). */
export function applyAnimationAtFrame(
  scene: Scene,
  animation: SceneAnimation,
  frame: number,
): Scene {
  let entities = scene.entities;
  for (const track of animation.tracks ?? []) {
    const sorted = [...track.keyframes].sort((a, b) => a.frame - b.frame);
    const value = sampleTrack(sorted, frame);
    if (value === null) continue;
    if (!findEntity(entities, track.entity)) continue;
    entities = updateEntity(entities, track.entity, (node) =>
      withProperty(node, track.property, value),
    );
  }
  return { ...scene, entities };
}

/** Highest keyframe frame of an animation (its duration). */
export function animationDuration(animation: SceneAnimation): number {
  let max = 0;
  for (const track of animation.tracks ?? []) {
    for (const keyframe of track.keyframes) {
      max = Math.max(max, keyframe.frame);
    }
  }
  return max;
}
