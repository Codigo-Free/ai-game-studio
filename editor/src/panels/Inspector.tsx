import { useEffect, useState } from "react";
import { findEntity, patchComponents, updateEntity } from "../document";
import { useStore } from "../store";
import type { Components } from "../types";

/** Numeric field that commits on blur/Enter (keeps undo history clean). */
function NumberField({
  label,
  value,
  step = 1,
  onCommit,
}: {
  label: string;
  value: number;
  step?: number;
  onCommit: (value: number) => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);

  const commit = () => {
    const parsed = Number(text);
    if (!Number.isNaN(parsed) && parsed !== value) onCommit(parsed);
    else setText(String(value));
  };

  return (
    <label className="field">
      <span>{label}</span>
      <input
        type="number"
        step={step}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
    </label>
  );
}

export function Inspector() {
  const { state, dispatch, currentScene } = useStore();
  const selection = state.selection;
  const node =
    currentScene && selection ? findEntity(currentScene.entities, selection) : null;

  if (!currentScene || !node) {
    return (
      <div className="panel inspector">
        <div className="panel-header">Inspector</div>
        <div className="panel-empty">Selecciona una entidad</div>
      </div>
    );
  }

  const patch = (componentsPatch: Partial<Components>) =>
    dispatch({
      type: "UPDATE_SCENE",
      scene: {
        ...currentScene,
        entities: patchComponents(currentScene.entities, node.id, componentsPatch),
      },
      commit: true,
    });

  const transform = node.components?.transform2d;
  const sprite = node.components?.sprite;
  const camera = node.components?.camera2d;
  const assets = state.loaded?.project.assets ?? [];
  const imageAssets = assets.filter((asset) => asset.kind === "image");

  return (
    <div className="panel inspector">
      <div className="panel-header">Inspector</div>
      <div className="panel-body">
        <label className="field">
          <span>Nombre</span>
          <input
            key={node.id}
            defaultValue={node.name}
            onBlur={(event) => {
              const name = event.target.value;
              if (name && name !== node.name) {
                dispatch({
                  type: "UPDATE_SCENE",
                  scene: {
                    ...currentScene,
                    entities: updateEntity(currentScene.entities, node.id, (n) => ({
                      ...n,
                      name,
                    })),
                  },
                  commit: true,
                });
              }
            }}
          />
        </label>
        <div className="field readonly">
          <span>Id</span>
          <code>{node.id}</code>
        </div>

        <section>
          <h4>
            Transform2D
            {!transform && (
              <button onClick={() => patch({ transform2d: {} })}>＋</button>
            )}
          </h4>
          {transform && (
            <div className="field-grid">
              <NumberField label="X" value={transform.x ?? 0} onCommit={(x) => patch({ transform2d: { ...transform, x } })} />
              <NumberField label="Y" value={transform.y ?? 0} onCommit={(y) => patch({ transform2d: { ...transform, y } })} />
              <NumberField label="Rotación" value={transform.rotation ?? 0} onCommit={(rotation) => patch({ transform2d: { ...transform, rotation } })} />
              <NumberField label="Escala X" value={transform.scale_x ?? 1} step={0.1} onCommit={(scale_x) => patch({ transform2d: { ...transform, scale_x } })} />
              <NumberField label="Escala Y" value={transform.scale_y ?? 1} step={0.1} onCommit={(scale_y) => patch({ transform2d: { ...transform, scale_y } })} />
            </div>
          )}
        </section>

        <section>
          <h4>
            Sprite
            {!sprite && imageAssets.length > 0 && (
              <button onClick={() => patch({ sprite: { asset: imageAssets[0].id } })}>＋</button>
            )}
            {sprite && (
              <button onClick={() => patch({ sprite: undefined })}>✕</button>
            )}
          </h4>
          {sprite && (
            <div className="field-grid">
              <label className="field">
                <span>Asset</span>
                <select
                  value={sprite.asset}
                  onChange={(event) => patch({ sprite: { ...sprite, asset: event.target.value } })}
                >
                  {imageAssets.map((asset) => (
                    <option key={asset.id} value={asset.id}>{asset.id}</option>
                  ))}
                </select>
              </label>
              <NumberField label="Opacidad" value={sprite.opacity ?? 1} step={0.05} onCommit={(opacity) => patch({ sprite: { ...sprite, opacity } })} />
              <NumberField label="Capa" value={sprite.layer ?? 0} onCommit={(layer) => patch({ sprite: { ...sprite, layer } })} />
              <NumberField label="Ancho" value={sprite.width ?? 0} onCommit={(width) => patch({ sprite: { ...sprite, width: width || undefined } })} />
              <NumberField label="Alto" value={sprite.height ?? 0} onCommit={(height) => patch({ sprite: { ...sprite, height: height || undefined } })} />
              <p className="hint">Ancho/alto 0 = tamaño de la textura</p>
            </div>
          )}
        </section>

        <section>
          <h4>
            Camera2D
            {!camera && (
              <button onClick={() => patch({ camera2d: { zoom: 1 } })}>＋</button>
            )}
            {camera && (
              <button onClick={() => patch({ camera2d: undefined })}>✕</button>
            )}
          </h4>
          {camera && (
            <div className="field-grid">
              <NumberField label="Zoom" value={camera.zoom ?? 1} step={0.1} onCommit={(zoom) => patch({ camera2d: { zoom } })} />
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
