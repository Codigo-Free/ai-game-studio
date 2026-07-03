// Scene viewport: Canvas 2D design view of the current scene.
//
// Architecture decision (M3): the edit view renders the document model
// directly on a canvas — same TRS math as the runtime — while Play mode
// launches the real WGPU runtime (`aigs run`). See docs/arquitectura.md.

import { useEffect, useRef, useState } from "react";
import {
  compose,
  generateId,
  IDENTITY,
  parentTransformOf,
  patchComponents,
  insertEntity,
  type WorldTransform,
} from "../document";
import { ensureImageUrl } from "./AssetsPanel";
import { snapshotOf, useStore } from "../store";
import type { Asset, EntityNode, SpriteComponent } from "../types";

interface ViewState {
  x: number;
  y: number;
  zoom: number;
}

interface RenderItem {
  id: string;
  world: WorldTransform;
  sprite: SpriteComponent;
  order: number;
}

function collectRenderList(
  entities: EntityNode[],
  parent: WorldTransform,
  into: RenderItem[],
): void {
  for (const node of entities) {
    const world = compose(parent, node.components?.transform2d);
    if (node.components?.sprite) {
      into.push({
        id: node.id,
        world,
        sprite: node.components.sprite,
        order: into.length,
      });
    }
    collectRenderList(node.children ?? [], world, into);
  }
}

function collectCameras(
  entities: EntityNode[],
  parent: WorldTransform,
  into: { id: string; world: WorldTransform }[],
): void {
  for (const node of entities) {
    const world = compose(parent, node.components?.transform2d);
    if (node.components?.camera2d) into.push({ id: node.id, world });
    collectCameras(node.children ?? [], world, into);
  }
}

export function Viewport() {
  const { state, dispatch, currentScene } = useStore();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [view, setView] = useState<ViewState>({ x: 0, y: 0, zoom: 1 });
  const [, setLoadedImages] = useState(0);
  const imagesRef = useRef(new Map<string, HTMLImageElement>());
  const dragRef = useRef<{
    id: string;
    lastX: number;
    lastY: number;
    snapshot: string;
    moved: boolean;
  } | null>(null);
  const panRef = useRef<{ lastX: number; lastY: number } | null>(null);

  const loaded = state.loaded;
  const assets = loaded?.project.assets ?? [];

  // Load sprite images into the cache as data URLs.
  useEffect(() => {
    if (!loaded) return;
    for (const asset of assets) {
      if (asset.kind !== "image" || imagesRef.current.has(asset.id)) continue;
      ensureImageUrl(loaded.root, asset)
        .then((url) => {
          const img = new Image();
          img.onload = () => setLoadedImages((n) => n + 1);
          img.src = url;
          imagesRef.current.set(asset.id, img);
        })
        .catch(() => {});
    }
  }, [loaded, assets]);

  const toWorld = (clientX: number, clientY: number) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const sx = clientX - rect.left;
    const sy = clientY - rect.top;
    return {
      x: view.x + (sx - rect.width / 2) / view.zoom,
      y: view.y + (rect.height / 2 - sy) / view.zoom,
    };
  };

  const spriteSize = (sprite: SpriteComponent) => {
    const img = imagesRef.current.get(sprite.asset);
    return {
      w: sprite.width ?? img?.naturalWidth ?? 64,
      h: sprite.height ?? img?.naturalHeight ?? 64,
    };
  };

  const hitTest = (worldX: number, worldY: number): string | null => {
    if (!currentScene) return null;
    const items: RenderItem[] = [];
    collectRenderList(currentScene.entities, IDENTITY, items);
    items.sort((a, b) => (a.sprite.layer ?? 0) - (b.sprite.layer ?? 0) || a.order - b.order);
    for (let i = items.length - 1; i >= 0; i -= 1) {
      const { world, sprite } = items[i];
      const { w, h } = spriteSize(sprite);
      const m = (world.rotation * Math.PI) / 180;
      const dx = worldX - world.x;
      const dy = worldY - world.y;
      const lx = (dx * Math.cos(m) - dy * Math.sin(m)) / (world.scaleX || 1);
      const ly = (dx * Math.sin(m) + dy * Math.cos(m)) / (world.scaleY || 1);
      if (Math.abs(lx) <= w / 2 && Math.abs(ly) <= h / 2) return items[i].id;
    }
    return null;
  };

  // ---- drawing ------------------------------------------------------------

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const parent = canvas.parentElement!;
    const dpr = window.devicePixelRatio || 1;
    const width = parent.clientWidth;
    const height = parent.clientHeight;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Background + grid.
    ctx.fillStyle = "#14151c";
    ctx.fillRect(0, 0, width, height);
    const gridStep = 64 * view.zoom;
    if (gridStep >= 12) {
      ctx.strokeStyle = "rgba(255,255,255,0.05)";
      ctx.lineWidth = 1;
      const offsetX = (width / 2 - view.x * view.zoom) % gridStep;
      const offsetY = (height / 2 + view.y * view.zoom) % gridStep;
      ctx.beginPath();
      for (let x = offsetX; x < width; x += gridStep) {
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
      }
      for (let y = offsetY; y < height; y += gridStep) {
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
      }
      ctx.stroke();
    }
    // Axes.
    const originX = width / 2 - view.x * view.zoom;
    const originY = height / 2 + view.y * view.zoom;
    ctx.strokeStyle = "rgba(120,140,255,0.25)";
    ctx.beginPath();
    ctx.moveTo(originX, 0);
    ctx.lineTo(originX, height);
    ctx.moveTo(0, originY);
    ctx.lineTo(width, originY);
    ctx.stroke();

    if (!currentScene) return;

    const toScreen = (wx: number, wy: number) => ({
      x: width / 2 + (wx - view.x) * view.zoom,
      y: height / 2 - (wy - view.y) * view.zoom,
    });

    // Sprites, sorted by layer (painter's algorithm, like the runtime).
    const items: RenderItem[] = [];
    collectRenderList(currentScene.entities, IDENTITY, items);
    items.sort((a, b) => (a.sprite.layer ?? 0) - (b.sprite.layer ?? 0) || a.order - b.order);
    for (const item of items) {
      const { world, sprite } = item;
      const img = imagesRef.current.get(sprite.asset);
      const { w, h } = spriteSize(sprite);
      const screen = toScreen(world.x, world.y);
      ctx.save();
      ctx.translate(screen.x, screen.y);
      ctx.rotate((world.rotation * Math.PI) / 180);
      ctx.scale(world.scaleX * view.zoom, world.scaleY * view.zoom);
      ctx.globalAlpha = sprite.opacity ?? 1;
      if (img && img.complete && img.naturalWidth > 0) {
        ctx.drawImage(img, -w / 2, -h / 2, w, h);
      } else {
        ctx.fillStyle = "#7f5af0";
        ctx.fillRect(-w / 2, -h / 2, w, h);
      }
      ctx.restore();

      if (item.id === state.selection) {
        ctx.save();
        ctx.translate(screen.x, screen.y);
        ctx.rotate((world.rotation * Math.PI) / 180);
        ctx.strokeStyle = "#4f9cff";
        ctx.lineWidth = 1.5;
        ctx.strokeRect(
          (-w / 2) * world.scaleX * view.zoom,
          (-h / 2) * world.scaleY * view.zoom,
          w * world.scaleX * view.zoom,
          h * world.scaleY * view.zoom,
        );
        ctx.restore();
      }
    }

    // Camera markers.
    const cameras: { id: string; world: WorldTransform }[] = [];
    collectCameras(currentScene.entities, IDENTITY, cameras);
    for (const camera of cameras) {
      const screen = toScreen(camera.world.x, camera.world.y);
      const selected = camera.id === state.selection;
      ctx.strokeStyle = selected ? "#4f9cff" : "rgba(255,255,255,0.4)";
      ctx.strokeRect(screen.x - 12, screen.y - 8, 24, 16);
      ctx.beginPath();
      ctx.moveTo(screen.x + 12, screen.y - 4);
      ctx.lineTo(screen.x + 18, screen.y - 8);
      ctx.lineTo(screen.x + 18, screen.y + 8);
      ctx.lineTo(screen.x + 12, screen.y + 4);
      ctx.stroke();
    }
  });

  // ---- interaction --------------------------------------------------------

  const onPointerDown = (event: React.PointerEvent) => {
    if (!currentScene) return;
    (event.target as Element).setPointerCapture(event.pointerId);
    if (event.button === 1 || event.shiftKey) {
      panRef.current = { lastX: event.clientX, lastY: event.clientY };
      return;
    }
    if (event.button !== 0) return;
    const world = toWorld(event.clientX, event.clientY);
    const hit = hitTest(world.x, world.y);
    dispatch({ type: "SELECT", id: hit });
    if (hit) {
      dragRef.current = {
        id: hit,
        lastX: event.clientX,
        lastY: event.clientY,
        snapshot: snapshotOf(state),
        moved: false,
      };
    }
  };

  const onPointerMove = (event: React.PointerEvent) => {
    if (panRef.current) {
      const dx = event.clientX - panRef.current.lastX;
      const dy = event.clientY - panRef.current.lastY;
      panRef.current = { lastX: event.clientX, lastY: event.clientY };
      setView((v) => ({ ...v, x: v.x - dx / v.zoom, y: v.y + dy / v.zoom }));
      return;
    }
    const drag = dragRef.current;
    if (!drag || !currentScene) return;
    const dxWorld = (event.clientX - drag.lastX) / view.zoom;
    const dyWorld = -(event.clientY - drag.lastY) / view.zoom;
    drag.lastX = event.clientX;
    drag.lastY = event.clientY;
    drag.moved = true;

    const parent =
      parentTransformOf(currentScene.entities, drag.id) ?? IDENTITY;
    const m = (parent.rotation * Math.PI) / 180;
    const localDx =
      (dxWorld * Math.cos(m) - dyWorld * Math.sin(m)) / (parent.scaleX || 1);
    const localDy =
      (dxWorld * Math.sin(m) + dyWorld * Math.cos(m)) / (parent.scaleY || 1);

    const entities = patchComponentsMove(currentScene.entities, drag.id, localDx, localDy);
    dispatch({
      type: "UPDATE_SCENE",
      scene: { ...currentScene, entities },
      commit: false,
    });
  };

  const onPointerUp = () => {
    panRef.current = null;
    const drag = dragRef.current;
    dragRef.current = null;
    if (drag?.moved) {
      dispatch({ type: "PUSH_HISTORY", snapshot: drag.snapshot });
    }
  };

  const onWheel = (event: React.WheelEvent) => {
    const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
    const cursor = toWorld(event.clientX, event.clientY);
    setView((v) => {
      const zoom = Math.min(8, Math.max(0.1, v.zoom * factor));
      return {
        zoom,
        x: cursor.x - (cursor.x - v.x) * (v.zoom / zoom),
        y: cursor.y - (cursor.y - v.y) * (v.zoom / zoom),
      };
    });
  };

  const onDrop = (event: React.DragEvent) => {
    event.preventDefault();
    if (!currentScene) return;
    const assetId = event.dataTransfer.getData("aigs/asset-id");
    const asset = assets.find((a: Asset) => a.id === assetId);
    if (!asset) return;
    const world = toWorld(event.clientX, event.clientY);
    const id = generateId(currentScene, asset.id);
    const node: EntityNode = {
      id,
      name: asset.id,
      components: {
        transform2d: { x: Math.round(world.x), y: Math.round(world.y) },
        sprite: { asset: asset.id },
      },
    };
    dispatch({
      type: "UPDATE_SCENE",
      scene: {
        ...currentScene,
        entities: insertEntity(currentScene.entities, null, node),
      },
      commit: true,
    });
    dispatch({ type: "SELECT", id });
    dispatch({
      type: "LOG",
      level: "info",
      message: `Sprite "${id}" añadido a la escena`,
    });
  };

  return (
    <div className="viewport">
      <canvas
        ref={canvasRef}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onWheel={onWheel}
        onDragOver={(event) => event.preventDefault()}
        onDrop={onDrop}
      />
      <div className="viewport-hud">
        {(view.zoom * 100).toFixed(0)}% — Shift+arrastrar o botón central: pan · rueda: zoom
      </div>
    </div>
  );
}

/** Applies a positional delta to an entity's local transform. */
function patchComponentsMove(
  entities: EntityNode[],
  id: string,
  dx: number,
  dy: number,
): EntityNode[] {
  let transform: { x?: number; y?: number } = {};
  const found = (function find(nodes: EntityNode[]): EntityNode | null {
    for (const node of nodes) {
      if (node.id === id) return node;
      const child = find(node.children ?? []);
      if (child) return child;
    }
    return null;
  })(entities);
  transform = found?.components?.transform2d ?? {};
  return patchComponents(entities, id, {
    transform2d: {
      ...found?.components?.transform2d,
      x: (transform.x ?? 0) + dx,
      y: (transform.y ?? 0) + dy,
    },
  });
}
