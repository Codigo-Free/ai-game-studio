import { useEffect, useState } from "react";
import { allEntityIds, findEntity, patchComponents, updateEntity } from "../document";
import { useStore } from "../store";
import { useRef } from "react";
import type {
  ActionSpec,
  AnimatorComponent,
  Behavior,
  BodyType,
  Components,
  EventSpec,
  ParticlesComponent,
  ShapeComponent,
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
          : act.type === "play_sound"
            ? `sonar "${act.asset}"`
            : `emitir ${act.count ?? 20} partículas`;
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
  const [burstCount, setBurstCount] = useState("20");

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
          : actionType === "play_sound"
            ? {
                type: "play_sound",
                asset: sound || audioAssets[0] || "",
              }
            : {
                type: "emit_particles",
                count: Math.max(1, Number(burstCount) || 20),
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
          <option value="emit_particles">emitir partículas</option>
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
        {actionType === "emit_particles" && (
          <input
            type="number"
            value={burstCount}
            onChange={(e) => setBurstCount(e.target.value)}
            style={{ width: 60 }}
            title="Cantidad de partículas (la entidad necesita el componente Partículas)"
          />
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

/** Miniature live simulation of the emitter (mirrors the runtime rules). */
function ParticlePreview({ particles }: { particles: ParticlesComponent }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const specRef = useRef(particles);
  specRef.current = particles;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    interface P { x: number; y: number; vx: number; vy: number; age: number }
    let pool: P[] = [];
    let accumulator = 0;
    let raf = 0;
    let last = performance.now();
    const step = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const spec = specRef.current;
      const lifetime = Math.max(0.05, spec.lifetime ?? 0.8);
      const rate = spec.emitting === false ? 0 : (spec.rate ?? 20);
      accumulator += rate * dt;
      // Idle emitters still preview a periodic burst so you can see something.
      if (rate === 0 && pool.length === 0) accumulator = 12;
      let toSpawn = Math.floor(accumulator);
      accumulator -= toSpawn;
      while (toSpawn-- > 0) {
        const arc = ((spec.spread ?? 360) * Math.PI) / 180;
        const angle = ((spec.direction ?? 90) * Math.PI) / 180 + (Math.random() - 0.5) * arc;
        const speed = (spec.speed ?? 120) * (0.6 + 0.4 * Math.random()) * 0.4;
        pool.push({ x: 0, y: 0, vx: Math.cos(angle) * speed, vy: Math.sin(angle) * speed, age: 0 });
      }
      pool = pool.filter((p) => (p.age += dt) < lifetime);
      ctx.fillStyle = "#14151c";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      for (const p of pool) {
        p.vy += (spec.gravity ?? 0) * dt * 0.4;
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        const t = p.age / lifetime;
        const scale = (spec.start_scale ?? 1) + ((spec.end_scale ?? 0.2) - (spec.start_scale ?? 1)) * t;
        const opacity = (spec.start_opacity ?? 1) + ((spec.end_opacity ?? 0) - (spec.start_opacity ?? 1)) * t;
        ctx.globalAlpha = Math.max(0, opacity);
        ctx.fillStyle = "#ffd85e";
        const size = Math.max(1, 5 * scale);
        ctx.fillRect(cx + p.x - size / 2, cy - p.y - size / 2, size, size);
      }
      ctx.globalAlpha = 1;
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, []);

  return <canvas ref={canvasRef} width={230} height={110} className="particle-preview" />;
}

function ParticlesSection({
  particles,
  imageAssets,
  onChange,
}: {
  particles?: ParticlesComponent;
  imageAssets: string[];
  onChange: (particles: ParticlesComponent | undefined) => void;
}) {
  return (
    <section>
      <h4>
        Partículas
        {!particles && (
          <button
            onClick={() => onChange({ asset: imageAssets[0] ?? "" })}
            disabled={imageAssets.length === 0}
          >
            ＋
          </button>
        )}
        {particles && <button onClick={() => onChange(undefined)}>✕</button>}
      </h4>
      {particles && (
        <div className="field-grid">
          <ParticlePreview particles={particles} />
          <label className="field">
            <span>Asset</span>
            <select
              value={particles.asset}
              onChange={(e) => onChange({ ...particles, asset: e.target.value })}
            >
              {imageAssets.map((id) => (
                <option key={id} value={id}>{id}</option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Emitir</span>
            <input
              type="checkbox"
              checked={particles.emitting ?? true}
              onChange={(e) => onChange({ ...particles, emitting: e.target.checked })}
            />
          </label>
          <NumberField label="Tasa/s" value={particles.rate ?? 20} onCommit={(rate) => onChange({ ...particles, rate: Math.max(0, rate) })} />
          <NumberField label="Vida (s)" value={particles.lifetime ?? 0.8} step={0.1} onCommit={(lifetime) => onChange({ ...particles, lifetime: Math.max(0.05, lifetime) })} />
          <NumberField label="Velocidad" value={particles.speed ?? 120} onCommit={(speed) => onChange({ ...particles, speed })} />
          <NumberField label="Dirección" value={particles.direction ?? 90} onCommit={(direction) => onChange({ ...particles, direction })} />
          <NumberField label="Apertura" value={particles.spread ?? 360} onCommit={(spread) => onChange({ ...particles, spread: Math.max(0, Math.min(360, spread)) })} />
          <NumberField label="Gravedad" value={particles.gravity ?? 0} onCommit={(gravity) => onChange({ ...particles, gravity })} />
          <NumberField label="Escala ini" value={particles.start_scale ?? 1} step={0.1} onCommit={(start_scale) => onChange({ ...particles, start_scale })} />
          <NumberField label="Escala fin" value={particles.end_scale ?? 0.2} step={0.1} onCommit={(end_scale) => onChange({ ...particles, end_scale })} />
          <NumberField label="Opac. ini" value={particles.start_opacity ?? 1} step={0.05} onCommit={(start_opacity) => onChange({ ...particles, start_opacity })} />
          <NumberField label="Opac. fin" value={particles.end_opacity ?? 0} step={0.05} onCommit={(end_opacity) => onChange({ ...particles, end_opacity })} />
          <NumberField label="Capa" value={particles.layer ?? 5} onCommit={(layer) => onChange({ ...particles, layer: Math.floor(layer) })} />
          <p className="hint">Con "Emitir" apagado, usa la acción "emitir partículas"</p>
        </div>
      )}
    </section>
  );
}

function ShapeSection({
  shape,
  onChange,
}: {
  shape?: ShapeComponent;
  onChange: (shape: ShapeComponent | undefined) => void;
}) {
  return (
    <section>
      <h4>
        Figura
        {!shape && <button onClick={() => onChange({})}>＋</button>}
        {shape && <button onClick={() => onChange(undefined)}>✕</button>}
      </h4>
      {shape && (
        <div className="field-grid">
          <label className="field">
            <span>Forma</span>
            <select
              value={shape.kind ?? "box"}
              onChange={(e) => onChange({ ...shape, kind: e.target.value as "box" | "circle" })}
            >
              <option value="box">caja</option>
              <option value="circle">círculo</option>
            </select>
          </label>
          {(shape.kind ?? "box") === "box" ? (
            <>
              <NumberField label="Ancho" value={shape.width ?? 40} onCommit={(width) => onChange({ ...shape, width })} />
              <NumberField label="Alto" value={shape.height ?? 40} onCommit={(height) => onChange({ ...shape, height })} />
            </>
          ) : (
            <NumberField label="Radio" value={shape.radius ?? 20} onCommit={(radius) => onChange({ ...shape, radius })} />
          )}
          <label className="field">
            <span>Color</span>
            <input
              type="color"
              value={shape.color ?? "#7f5af0"}
              onChange={(e) => onChange({ ...shape, color: e.target.value })}
            />
          </label>
          <NumberField label="Opacidad" value={shape.opacity ?? 1} step={0.05} onCommit={(opacity) => onChange({ ...shape, opacity })} />
          <NumberField label="Capa" value={shape.layer ?? 0} onCommit={(layer) => onChange({ ...shape, layer: Math.floor(layer) })} />
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

        <section>
          <h4>
            Script
            {!node.components?.script && (
              <button
                onClick={() => {
                  const scripts = (state.loaded?.project.assets ?? [])
                    .filter((a) => a.kind === "script")
                    .map((a) => a.id);
                  if (scripts.length > 0) patch({ script: { asset: scripts[0] } });
                }}
                disabled={(state.loaded?.project.assets ?? []).every((a) => a.kind !== "script")}
                title="Importa un archivo .rhai en Recursos primero"
              >
                ＋
              </button>
            )}
            {node.components?.script && (
              <button onClick={() => patch({ script: undefined })}>✕</button>
            )}
          </h4>
          {node.components?.script && (
            <div className="field-grid">
              <label className="field">
                <span>Asset</span>
                <select
                  value={node.components.script.asset}
                  onChange={(e) => patch({ script: { asset: e.target.value } })}
                >
                  {(state.loaded?.project.assets ?? [])
                    .filter((a) => a.kind === "script")
                    .map((a) => (
                      <option key={a.id} value={a.id}>{a.id}</option>
                    ))}
                </select>
              </label>
              <p className="hint">
                fn on_start() y fn on_update(dt); errores del script en la Consola al hacer Play
              </p>
            </div>
          )}
        </section>

        <ParticlesSection
          particles={node.components?.particles}
          imageAssets={(state.loaded?.project.assets ?? [])
            .filter((a) => a.kind === "image")
            .map((a) => a.id)}
          onChange={(particles) => patch({ particles })}
        />

        <ShapeSection
          shape={node.components?.shape}
          onChange={(shape) => patch({ shape })}
        />

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
