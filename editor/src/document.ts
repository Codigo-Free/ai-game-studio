// Pure helpers over the document model (immutable scene-tree operations).

import type { Components, EntityNode, Scene, Transform2D } from "./types";

export interface WorldTransform {
  x: number;
  y: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
}

export const IDENTITY: WorldTransform = {
  x: 0,
  y: 0,
  rotation: 0,
  scaleX: 1,
  scaleY: 1,
};

/** 2D TRS composition, mirroring `aigs-runtime`'s scene instantiation. */
export function compose(
  parent: WorldTransform,
  local: Transform2D | undefined,
): WorldTransform {
  const lx = local?.x ?? 0;
  const ly = local?.y ?? 0;
  const radians = (parent.rotation * Math.PI) / 180;
  const sin = Math.sin(radians);
  const cos = Math.cos(radians);
  const sx = lx * parent.scaleX;
  const sy = ly * parent.scaleY;
  return {
    x: parent.x + sx * cos - sy * sin,
    y: parent.y + sx * sin + sy * cos,
    rotation: parent.rotation + (local?.rotation ?? 0),
    scaleX: parent.scaleX * (local?.scale_x ?? 1),
    scaleY: parent.scaleY * (local?.scale_y ?? 1),
  };
}

export function findEntity(
  entities: EntityNode[],
  id: string,
): EntityNode | null {
  for (const node of entities) {
    if (node.id === id) return node;
    const inChild = findEntity(node.children ?? [], id);
    if (inChild) return inChild;
  }
  return null;
}

/** World transform of an entity (parents composed), or null if not found. */
export function worldTransformOf(
  entities: EntityNode[],
  id: string,
  parent: WorldTransform = IDENTITY,
): WorldTransform | null {
  for (const node of entities) {
    const world = compose(parent, node.components?.transform2d);
    if (node.id === id) return world;
    const inChild = worldTransformOf(node.children ?? [], id, world);
    if (inChild) return inChild;
  }
  return null;
}

/** Parent world transform of an entity (identity for root entities). */
export function parentTransformOf(
  entities: EntityNode[],
  id: string,
  parent: WorldTransform = IDENTITY,
): WorldTransform | null {
  for (const node of entities) {
    if (node.id === id) return parent;
    const world = compose(parent, node.components?.transform2d);
    const inChild = parentTransformOf(node.children ?? [], id, world);
    if (inChild) return inChild;
  }
  return null;
}

export function updateEntity(
  entities: EntityNode[],
  id: string,
  update: (node: EntityNode) => EntityNode,
): EntityNode[] {
  return entities.map((node) => {
    if (node.id === id) return update(node);
    const children = node.children;
    if (!children || children.length === 0) return node;
    const updated = updateEntity(children, id, update);
    return updated === children ? node : { ...node, children: updated };
  });
}

export function patchComponents(
  entities: EntityNode[],
  id: string,
  patch: Partial<Components>,
): EntityNode[] {
  return updateEntity(entities, id, (node) => ({
    ...node,
    components: { ...node.components, ...patch },
  }));
}

export function removeEntity(
  entities: EntityNode[],
  id: string,
): EntityNode[] {
  return entities
    .filter((node) => node.id !== id)
    .map((node) =>
      node.children && node.children.length > 0
        ? { ...node, children: removeEntity(node.children, id) }
        : node,
    );
}

/** Inserts under `parentId`, or at the root when `parentId` is null. */
export function insertEntity(
  entities: EntityNode[],
  parentId: string | null,
  child: EntityNode,
): EntityNode[] {
  if (parentId === null) return [...entities, child];
  return updateEntity(entities, parentId, (node) => ({
    ...node,
    children: [...(node.children ?? []), child],
  }));
}

/** Moves an entity up/down among its siblings. */
export function reorderEntity(
  entities: EntityNode[],
  id: string,
  direction: -1 | 1,
): EntityNode[] {
  const index = entities.findIndex((node) => node.id === id);
  if (index !== -1) {
    const target = index + direction;
    if (target < 0 || target >= entities.length) return entities;
    const copy = [...entities];
    [copy[index], copy[target]] = [copy[target], copy[index]];
    return copy;
  }
  return entities.map((node) =>
    node.children && node.children.length > 0
      ? { ...node, children: reorderEntity(node.children, id, direction) }
      : node,
  );
}

export function collectIds(entities: EntityNode[], into: Set<string>): void {
  for (const node of entities) {
    into.add(node.id);
    collectIds(node.children ?? [], into);
  }
}

/** Generates a unique, format-friendly id from a display name. */
export function generateId(scene: Scene, name: string): string {
  const base =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "entity";
  const taken = new Set<string>();
  collectIds(scene.entities, taken);
  if (!taken.has(base)) return base;
  let counter = 2;
  while (taken.has(`${base}-${counter}`)) counter += 1;
  return `${base}-${counter}`;
}
