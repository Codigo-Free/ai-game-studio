import { useEffect, useState } from "react";
import { allEntityIds, findEntity, patchComponents, updateEntity } from "../document";
import { useStore } from "../store";
import type {
  ActionSpec,
  AnimatorComponent,
  Behavior,
  BodyType,
  Components,
  EventSpec,
} from "../types";

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
        : on.type === "key_released"
          ? `tecla ${on.key} (soltada)`
          : on.type === "click"
          ? "clic"
          : on.type === "scene_start"
            ? "inicio de escena"
            : on.type === "animation_end"
              ? `fin de "${on.animation}"`
              : on.with
                ? `choca con ${on.with}`
                : "choca con algo";
  const act = behavior.do;
  const action =
    act.type === "move"
      ? `mover (${act.dx}, ${act.dy})`
      : act.type === "goto_scene"
        ? `ir a ${act.scene.split("/").pop()?.replace(".scene.aigs", "")}`
        : act.type === "play_animation"
          ? `animar "${act.animation}"`
          : `sonar "${act.asset}"`;
  return `${event} → ${action}`;
}

/** Small form to append a new behavior to the selected entity. */
function BehaviorForm({
  scenes,
  animations,
  entityIds,
  audioAssets,
  onAdd,
}: {
  scenes: string[];
  animations: string[];
  entityIds: string[];
  audioAssets: string[];
  onAdd: (behavior: Behavior) => void;
}) {
  const [eventType, setEventType] = useState<EventSpec["type"]>("key_down");
  const [key, setKey] = useState("ArrowRight");
  const [eventAnim, setEventAnim] = useState("");
  const [collisionWith, setCollisionWith] = useState("");
  const [actionType, setActionType] = useState<ActionSpec["type"]>("move");
  const [dx, setDx] = useState("200");
  const [dy, setDy] = useState("0");
  const [scene, setScene] = useState(scenes[0] ?? "");
  const [actionAnim, setActionAnim] = useState("");
  const [sound, setSound] = useState("");

  const add = () => {
    const on: EventSpec =
      eventType === "key_down" || eventType === "key_pressed" || eventType === "key_released"
        ? { type: eventType, key }
        : eventType === "animation_end"
          ? { type: "animation_end", animation: eventAnim || animations[0] || "" }
          : eventType === "collision"
            ? collisionWith
              ? { type: "collision", with: collisionWith }
              : { type: "collision" }
            : { type: eventType };
    const run: ActionSpec =
      actionType === "move"
        ? { type: "move", dx: Number(dx) || 0, dy: Number(dy) || 0 }
        : actionType === "goto_scene"
          ? { type: "goto_scene", scene: scene || scenes[0] || "" }
          : actionType === "play_animation"
          ? {
              type: "play_animation",
              animation: actionAnim || animations[0] || "",
            }
          : {
              type: "play_sound",
              asset: sound || audioAssets[0] || "",
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
          <option value="key_released">tecla soltada</option>
          <option value="click">clic en la entidad</option>
          <option value="scene_start">inicia la escena</option>
          <option value="animation_end">termina animación</option>
          <option value="collision">colisiona</option>
        </select>
        {eventType === "collision" && (
          <select value={collisionWith} onChange={(e) => setCollisionWith(e.target.value)}>
            <option value="">con cualquiera</option>
            {entityIds.map((id) => (
              <option key={id} value={id}>con {id}</option>
            ))}
          </select>
        )}
        {(eventType === "key_down" || eventType === "key_pressed" || eventType === "key_released") && (
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
          <option value="play_sound">reproducir sonido</option>
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
        {actionType === "play_sound" && (
          <select value={sound} onChange={(e) => setSound(e.target.value)}>
            {audioAssets.length === 0 && <option value="">— importa un audio —</option>}
            {audioAssets.map((id) => (
              <option key={id} value={id}>{id}</option>
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

function AnimatorSection({
  animator,
  animations,
  onChange,
}: {
  animator?: AnimatorComponent;
  animations: string[];
  onChange: (animator: AnimatorComponent | undefined) => void;
}) {
  const [newState, setNewState] = useState("");
  const [transitionFrom, setTransitionFrom] = useState("");
  const [transitionTo, setTransitionTo] = useState("");
  const [transitionKey, setTransitionKey] = useState("ArrowRight");
  const [transitionEvent, setTransitionEvent] = useState("key_down");

  return (
    <section>
      <h4>
        Animator
        {!animator && (
          <button
            onClick={() =>
              onChange({ initial: "idle", states: { idle: animations[0] ?? "" }, transitions: [] })
            }
            disabled={animations.length === 0}
            title={animations.length === 0 ? "Crea animaciones en el Timeline primero" : ""}
          >
            ＋
          </button>
        )}
        {animator && <button onClick={() => onChange(undefined)}>✕</button>}
      </h4>
      {animator && (
        <div className="field-grid">
          <label className="field">
            <span>Inicial</span>
            <select
              value={animator.initial}
              onChange={(e) => onChange({ ...animator, initial: e.target.value })}
            >
              {Object.keys(animator.states).map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
          </label>
          {Object.entries(animator.states).map(([name, animation]) => (
            <div key={name} className="behavior-row">
              <span className="behavior-text">{name} →</span>
              <select
                value={animation}
                onChange={(e) =>
                  onChange({ ...animator, states: { ...animator.states, [name]: e.target.value } })
                }
              >
                {animations.map((a) => (
                  <option key={a} value={a}>{a}</option>
                ))}
              </select>
              <button
                onClick={() => {
                  const states = { ...animator.states };
                  delete states[name];
                  onChange({
                    ...animator,
                    states,
                    transitions: (animator.transitions ?? []).filter(
                      (t) => t.from !== name && t.to !== name,
                    ),
                  });
                }}
              >
                ✕
              </button>
            </div>
          ))}
          <div className="behavior-form-row">
            <input
              placeholder="nuevo estado…"
              value={newState}
              onChange={(e) => setNewState(e.target.value)}
              style={{ width: 110 }}
            />
            <button
              onClick={() => {
                const name = newState.trim();
                if (!name || animator.states[name]) return;
                onChange({
                  ...animator,
                  states: { ...animator.states, [name]: animations[0] ?? "" },
                });
                setNewState("");
              }}
            >
              ＋ estado
            </button>
          </div>
          {(animator.transitions ?? []).map((transition, index) => (
            <div key={index} className="behavior-row">
              <span className="behavior-text">
                {transition.from} → {transition.to}
                {" si "}
                {"key" in transition.when
                  ? `${transition.when.key} (${transition.when.type.replace("key_", "")})`
                  : transition.when.type}
              </span>
              <button
                onClick={() =>
                  onChange({
                    ...animator,
                    transitions: (animator.transitions ?? []).filter((_, i) => i !== index),
                  })
                }
              >
                ✕
              </button>
            </div>
          ))}
          <div className="behavior-form-row">
            <select value={transitionFrom} onChange={(e) => setTransitionFrom(e.target.value)}>
              <option value="">de…</option>
              <option value="any">any</option>
              {Object.keys(animator.states).map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <select value={transitionTo} onChange={(e) => setTransitionTo(e.target.value)}>
              <option value="">a…</option>
              {Object.keys(animator.states).map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <select value={transitionEvent} onChange={(e) => setTransitionEvent(e.target.value)}>
              <option value="key_down">tecla mantenida</option>
              <option value="key_pressed">tecla pulsada</option>
              <option value="key_released">tecla soltada</option>
            </select>
            <input
              list="common-keys"
              value={transitionKey}
              onChange={(e) => setTransitionKey(e.target.value)}
              style={{ width: 90 }}
            />
            <button
              onClick={() => {
                if (!transitionFrom || !transitionTo) return;
                onChange({
                  ...animator,
                  transitions: [
                    ...(animator.transitions ?? []),
                    {
                      from: transitionFrom,
                      to: transitionTo,
                      when: {
                        type: transitionEvent as "key_down" | "key_pressed" | "key_released",
                        key: transitionKey,
                      },
                    },
                  ],
                });
              }}
            >
              ＋ transición
            </button>
          </div>
          <p className="hint">Las animaciones usadas por estados no se auto-reproducen</p>
        </div>
      )}
    </section>
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
        {currentScene ? (
          <div className="panel-body">
            <section>
              <h4>Escena: {currentScene.name}</h4>
              <p className="hint">Gravedad (unidades/s², afecta a cuerpos dinámicos)</p>
              <div className="field-grid">
                <NumberField
                  label="Gravedad X"
                  value={currentScene.gravity?.x ?? 0}
                  onCommit={(x) =>
                    dispatch({
                      type: "UPDATE_SCENE",
                      scene: {
                        ...currentScene,
                        gravity: { ...currentScene.gravity, x, y: currentScene.gravity?.y ?? -980 },
                      },
                      commit: true,
                    })
                  }
                />
                <NumberField
                  label="Gravedad Y"
                  value={currentScene.gravity?.y ?? -980}
                  onCommit={(y) =>
                    dispatch({
                      type: "UPDATE_SCENE",
                      scene: {
                        ...currentScene,
                        gravity: { ...currentScene.gravity, x: currentScene.gravity?.x ?? 0, y },
                      },
                      commit: true,
                    })
                  }
                />
              </div>
              <p className="hint">Música de fondo de la escena</p>
              <div className="field-grid">
                <label className="field">
                  <span>Música</span>
                  <select
                    value={currentScene.music?.asset ?? ""}
                    onChange={(e) =>
                      dispatch({
                        type: "UPDATE_SCENE",
                        scene: {
                          ...currentScene,
                          music: e.target.value
                            ? { volume: 0.8, looped: true, ...currentScene.music, asset: e.target.value }
                            : undefined,
                        },
                        commit: true,
                      })
                    }
                  >
                    <option value="">— sin música —</option>
                    {(state.loaded?.project.assets ?? [])
                      .filter((a) => a.kind === "audio")
                      .map((a) => (
                        <option key={a.id} value={a.id}>{a.id}</option>
                      ))}
                  </select>
                </label>
                {currentScene.music && (
                  <>
                    <NumberField
                      label="Volumen"
                      value={currentScene.music.volume ?? 1}
                      step={0.05}
                      onCommit={(volume) =>
                        dispatch({
                          type: "UPDATE_SCENE",
                          scene: { ...currentScene, music: { ...currentScene.music!, volume } },
                          commit: true,
                        })
                      }
                    />
                    <label className="field">
                      <span>Loop</span>
                      <input
                        type="checkbox"
                        checked={currentScene.music.looped ?? true}
                        onChange={(e) =>
                          dispatch({
                            type: "UPDATE_SCENE",
                            scene: { ...currentScene, music: { ...currentScene.music!, looped: e.target.checked } },
                            commit: true,
                          })
                        }
                      />
                    </label>
                  </>
                )}
              </div>
              <p className="hint">Selecciona una entidad para editar sus componentes</p>
            </section>
          </div>
        ) : (
          <div className="panel-empty">Selecciona una entidad</div>
        )}
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
              <NumberField label="Frame" value={sprite.frame ?? 0} onCommit={(frame) => patch({ sprite: { ...sprite, frame: Math.max(0, Math.floor(frame)) } })} />
              <NumberField label="Opacidad" value={sprite.opacity ?? 1} step={0.05} onCommit={(opacity) => patch({ sprite: { ...sprite, opacity } })} />
              <NumberField label="Capa" value={sprite.layer ?? 0} onCommit={(layer) => patch({ sprite: { ...sprite, layer } })} />
              <NumberField label="Ancho" value={sprite.width ?? 0} onCommit={(width) => patch({ sprite: { ...sprite, width: width || undefined } })} />
              <NumberField label="Alto" value={sprite.height ?? 0} onCommit={(height) => patch({ sprite: { ...sprite, height: height || undefined } })} />
              <p className="hint">Ancho/alto 0 = tamaño de la textura</p>
            </div>
          )}
        </section>

        <AnimatorSection
          animator={node.components?.animator}
          animations={(currentScene.animations ?? []).map((a) => a.name)}
          onChange={(animator) => patch({ animator })}
        />

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
            entityIds={allEntityIds(currentScene.entities).filter((id) => id !== node.id)}
            audioAssets={(state.loaded?.project.assets ?? [])
              .filter((a) => a.kind === "audio")
              .map((a) => a.id)}
            onAdd={(behavior) =>
              patch({
                behaviors: [...(node.components?.behaviors ?? []), behavior],
              })
            }
          />
        </section>

        <section>
          <h4>
            Rigidbody2D
            {!node.components?.rigidbody2d && (
              <button onClick={() => patch({ rigidbody2d: {} })}>＋</button>
            )}
            {node.components?.rigidbody2d && (
              <button onClick={() => patch({ rigidbody2d: undefined })}>✕</button>
            )}
          </h4>
          {node.components?.rigidbody2d && (() => {
            const body = node.components.rigidbody2d!;
            return (
              <div className="field-grid">
                <label className="field">
                  <span>Tipo</span>
                  <select
                    value={body.body ?? "dynamic"}
                    onChange={(e) =>
                      patch({ rigidbody2d: { ...body, body: e.target.value as BodyType } })
                    }
                  >
                    <option value="dynamic">dinámico</option>
                    <option value="kinematic">kinemático</option>
                    <option value="static">estático</option>
                  </select>
                </label>
                <NumberField label="Grav. escala" value={body.gravity_scale ?? 1} step={0.1} onCommit={(gravity_scale) => patch({ rigidbody2d: { ...body, gravity_scale } })} />
                <NumberField label="Vel. X" value={body.vx ?? 0} onCommit={(vx) => patch({ rigidbody2d: { ...body, vx } })} />
                <NumberField label="Vel. Y" value={body.vy ?? 0} onCommit={(vy) => patch({ rigidbody2d: { ...body, vy } })} />
                <label className="field">
                  <span>Sin rotación</span>
                  <input
                    type="checkbox"
                    checked={body.fixed_rotation ?? false}
                    onChange={(e) => patch({ rigidbody2d: { ...body, fixed_rotation: e.target.checked } })}
                  />
                </label>
              </div>
            );
          })()}
        </section>

        <section>
          <h4>
            Collider2D
            {!node.components?.collider2d && (
              <button onClick={() => patch({ collider2d: {} })}>＋</button>
            )}
            {node.components?.collider2d && (
              <button onClick={() => patch({ collider2d: undefined })}>✕</button>
            )}
          </h4>
          {node.components?.collider2d && (() => {
            const collider = node.components.collider2d!;
            return (
              <div className="field-grid">
                <label className="field">
                  <span>Forma</span>
                  <select
                    value={collider.shape ?? "box"}
                    onChange={(e) =>
                      patch({ collider2d: { ...collider, shape: e.target.value as "box" | "circle" } })
                    }
                  >
                    <option value="box">caja</option>
                    <option value="circle">círculo</option>
                  </select>
                </label>
                {(collider.shape ?? "box") === "box" ? (
                  <>
                    <NumberField label="Ancho" value={collider.width ?? 0} onCommit={(width) => patch({ collider2d: { ...collider, width: width || undefined } })} />
                    <NumberField label="Alto" value={collider.height ?? 0} onCommit={(height) => patch({ collider2d: { ...collider, height: height || undefined } })} />
                  </>
                ) : (
                  <NumberField label="Radio" value={collider.radius ?? 0} onCommit={(radius) => patch({ collider2d: { ...collider, radius: radius || undefined } })} />
                )}
                <NumberField label="Rebote" value={collider.restitution ?? 0} step={0.05} onCommit={(restitution) => patch({ collider2d: { ...collider, restitution } })} />
                <NumberField label="Fricción" value={collider.friction ?? 0.5} step={0.05} onCommit={(friction) => patch({ collider2d: { ...collider, friction } })} />
                <label className="field">
                  <span>Sensor</span>
                  <input
                    type="checkbox"
                    checked={collider.sensor ?? false}
                    onChange={(e) => patch({ collider2d: { ...collider, sensor: e.target.checked } })}
                  />
                </label>
                <p className="hint">Tamaño 0 = tamaño visible del sprite</p>
              </div>
            );
          })()}
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
