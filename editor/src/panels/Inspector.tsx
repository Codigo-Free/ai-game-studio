import { useEffect, useState } from "react";
import { findEntity, patchComponents, updateEntity } from "../document";
import { useStore } from "../store";
import type { ActionSpec, Behavior, Components, EventSpec } from "../types";

const COMMON_KEYS = [
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "ArrowDown",
  "Space",
  "Enter",
  "Escape",
];

function describeBehavior(behavior: Behavior): string {
  const on = behavior.on;
  const event =
    on.type === "key_down"
      ? `tecla ${on.key} (mantenida)`
      : on.type === "key_pressed"
        ? `tecla ${on.key}`
        : on.type === "click"
          ? "clic"
          : on.type === "scene_start"
            ? "inicio de escena"
            : `fin de "${on.animation}"`;
  const act = behavior.do;
  const action =
    act.type === "move"
      ? `mover (${act.dx}, ${act.dy})`
      : act.type === "goto_scene"
        ? `ir a ${act.scene.split("/").pop()?.replace(".scene.aigs", "")}`
        : `animar "${act.animation}"`;
  return `${event} → ${action}`;
}

/** Small form to append a new behavior to the selected entity. */
function BehaviorForm({
  scenes,
  animations,
  onAdd,
}: {
  scenes: string[];
  animations: string[];
  onAdd: (behavior: Behavior) => void;
}) {
  const [eventType, setEventType] = useState<EventSpec["type"]>("key_down");
  const [key, setKey] = useState("ArrowRight");
  const [eventAnim, setEventAnim] = useState("");
  const [actionType, setActionType] = useState<ActionSpec["type"]>("move");
  const [dx, setDx] = useState("200");
  const [dy, setDy] = useState("0");
  const [scene, setScene] = useState(scenes[0] ?? "");
  const [actionAnim, setActionAnim] = useState("");

  const add = () => {
    const on: EventSpec =
      eventType === "key_down" || eventType === "key_pressed"
        ? { type: eventType, key }
        : eventType === "animation_end"
          ? { type: "animation_end", animation: eventAnim || animations[0] || "" }
          : { type: eventType };
    const run: ActionSpec =
      actionType === "move"
        ? { type: "move", dx: Number(dx) || 0, dy: Number(dy) || 0 }
        : actionType === "goto_scene"
          ? { type: "goto_scene", scene: scene || scenes[0] || "" }
          : {
              type: "play_animation",
              animation: actionAnim || animations[0] || "",
            };
    onAdd({ on, do: run });
  };

  return (
    <div className="behavior-form">
      <div className="behavior-form-row">
        <span>Cuando</span>
        <select value={eventType} onChange={(e) => setEventType(e.target.value as EventSpec["type"])}>
          <option value="key_down">tecla mantenida</option>
          <option value="key_pressed">tecla pulsada</option>
          <option value="click">clic en la entidad</option>
          <option value="scene_start">inicia la escena</option>
          <option value="animation_end">termina animación</option>
        </select>
        {(eventType === "key_down" || eventType === "key_pressed") && (
          <input
            list="common-keys"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            style={{ width: 90 }}
          />
        )}
        {eventType === "animation_end" && (
          <select value={eventAnim} onChange={(e) => setEventAnim(e.target.value)}>
            {animations.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        )}
        <datalist id="common-keys">
          {COMMON_KEYS.map((k) => (
            <option key={k} value={k} />
          ))}
        </datalist>
      </div>
      <div className="behavior-form-row">
        <span>hacer</span>
        <select value={actionType} onChange={(e) => setActionType(e.target.value as ActionSpec["type"])}>
          <option value="move">mover</option>
          <option value="goto_scene">ir a escena</option>
          <option value="play_animation">reproducir animación</option>
        </select>
        {actionType === "move" && (
          <>
            <input type="number" value={dx} onChange={(e) => setDx(e.target.value)} style={{ width: 60 }} title="dx (unidades/s si es continua)" />
            <input type="number" value={dy} onChange={(e) => setDy(e.target.value)} style={{ width: 60 }} title="dy" />
          </>
        )}
        {actionType === "goto_scene" && (
          <select value={scene} onChange={(e) => setScene(e.target.value)}>
            {scenes.map((path) => (
              <option key={path} value={path}>{path.split("/").pop()}</option>
            ))}
          </select>
        )}
        {actionType === "play_animation" && (
          <select value={actionAnim} onChange={(e) => setActionAnim(e.target.value)}>
            {animations.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        )}
        <button onClick={add}>Añadir</button>
      </div>
    </div>
  );
}

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
          <h4>Comportamientos</h4>
          {(node.components?.behaviors ?? []).map((behavior, index) => (
            <div key={index} className="behavior-row">
              <span className="behavior-text">{describeBehavior(behavior)}</span>
              <button
                onClick={() =>
                  patch({
                    behaviors: (node.components?.behaviors ?? []).filter(
                      (_, i) => i !== index,
                    ),
                  })
                }
                title="Eliminar comportamiento"
              >
                ✕
              </button>
            </div>
          ))}
          <BehaviorForm
            scenes={state.loaded?.project.scenes ?? []}
            animations={(currentScene.animations ?? []).map((a) => a.name)}
            onAdd={(behavior) =>
              patch({
                behaviors: [...(node.components?.behaviors ?? []), behavior],
              })
            }
          />
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
