// Timeline panel (milestone M4): Flash-style keyframe editing with
// scrubbing and in-viewport playback preview.

import { useEffect, useRef, useState } from "react";
import {
  ANIMATABLE_PROPERTIES,
  animationDuration,
  applyAnimationAtFrame,
  propertyValue,
  sampleTrack,
} from "../anim";
import { findEntity } from "../document";
import { snapshotOf, useStore } from "../store";
import type { Keyframe, Scene, SceneAnimation, Track } from "../types";

const FRAME_W = 12;
const RULER_STEP = 10;

export function Timeline() {
  const { state, dispatch, currentScene } = useStore();
  const [animIndex, setAnimIndex] = useState(0);
  const [playhead, setPlayhead] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [selectedKf, setSelectedKf] = useState<{
    track: number;
    kf: number;
  } | null>(null);
  const lanesRef = useRef<HTMLDivElement>(null);
  const dragKfRef = useRef<{
    track: number;
    kf: number;
    snapshot: string;
    moved: boolean;
  } | null>(null);
  const scrubbingRef = useRef(false);

  const animations = currentScene?.animations ?? [];
  const animation: SceneAnimation | undefined = animations[animIndex];
  const duration = animation ? animationDuration(animation) : 0;
  const maxFrames = Math.max(90, duration + 31);

  // Reset local state when the scene changes.
  useEffect(() => {
    setAnimIndex(0);
    setPlayhead(0);
    setPlaying(false);
    setSelectedKf(null);
  }, [state.currentScenePath]);

  const updateAnimations = (next: SceneAnimation[], commit: boolean) => {
    if (!currentScene) return;
    dispatch({
      type: "UPDATE_SCENE",
      scene: { ...currentScene, animations: next },
      commit,
    });
  };

  const updateAnimation = (
    next: SceneAnimation,
    commit: boolean,
    base?: Scene,
  ) => {
    const scene = base ?? currentScene;
    if (!scene) return;
    const list = [...(scene.animations ?? [])];
    list[animIndex] = next;
    dispatch({
      type: "UPDATE_SCENE",
      scene: { ...scene, animations: list },
      commit,
    });
  };

  const showPreviewAt = (frame: number, anim = animation) => {
    if (!currentScene || !anim) return;
    dispatch({
      type: "SET_PREVIEW",
      scene: applyAnimationAtFrame(currentScene, anim, frame),
    });
  };

  // Playback loop.
  useEffect(() => {
    if (!playing || !animation || !currentScene) return;
    let raf = 0;
    let last = performance.now();
    let frame = playhead;
    const step = (now: number) => {
      const dt = (now - last) / 1000;
      last = now;
      frame += dt * animation.fps;
      const total = animationDuration(animation);
      if (frame >= total) {
        if (animation.loop && total > 0) {
          frame %= total;
        } else {
          frame = total;
          setPlaying(false);
        }
      }
      setPlayhead(frame);
      showPreviewAt(frame);
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, animation, currentScene]);

  if (!currentScene) return <div className="panel-empty">Sin escena</div>;

  const stop = () => {
    setPlaying(false);
    setPlayhead(0);
    dispatch({ type: "SET_PREVIEW", scene: null });
  };

  const addAnimation = () => {
    const name = window.prompt("Nombre de la animación", "animacion");
    if (!name) return;
    updateAnimations(
      [...animations, { name, fps: 30, loop: false, tracks: [] }],
      true,
    );
    setAnimIndex(animations.length);
  };

  const deleteAnimation = () => {
    if (!animation) return;
    updateAnimations(animations.filter((_, i) => i !== animIndex), true);
    setAnimIndex(0);
    stop();
  };

  const addTrack = (property: string) => {
    if (!animation || !state.selection) return;
    const node = findEntity(currentScene.entities, state.selection);
    if (!node) return;
    const exists = (animation.tracks ?? []).some(
      (t) => t.entity === state.selection && t.property === property,
    );
    if (exists) {
      dispatch({ type: "LOG", level: "warn", message: "Esa pista ya existe" });
      return;
    }
    const track: Track = {
      entity: state.selection,
      property,
      keyframes: [
        {
          frame: Math.round(playhead),
          value: propertyValue(node, property),
          easing: "linear",
        },
      ],
    };
    updateAnimation(
      { ...animation, tracks: [...(animation.tracks ?? []), track] },
      true,
    );
  };

  const removeTrack = (index: number) => {
    if (!animation) return;
    updateAnimation(
      {
        ...animation,
        tracks: (animation.tracks ?? []).filter((_, i) => i !== index),
      },
      true,
    );
    setSelectedKf(null);
  };

  const frameFromEvent = (event: React.MouseEvent): number => {
    const lanes = lanesRef.current;
    if (!lanes) return 0;
    const rect = lanes.getBoundingClientRect();
    const x = event.clientX - rect.left + lanes.scrollLeft;
    return Math.max(0, x / FRAME_W);
  };

  const scrubTo = (event: React.MouseEvent) => {
    const frame = frameFromEvent(event);
    setPlayhead(frame);
    showPreviewAt(frame);
  };

  const addKeyframe = (trackIndex: number, frame: number) => {
    if (!animation) return;
    const track = (animation.tracks ?? [])[trackIndex];
    const rounded = Math.round(frame);
    if (track.keyframes.some((k) => k.frame === rounded)) return;
    const sorted = [...track.keyframes].sort((a, b) => a.frame - b.frame);
    const node = findEntity(currentScene.entities, track.entity);
    const value =
      sampleTrack(sorted, rounded) ??
      (node ? propertyValue(node, track.property) : 0);
    const keyframes = [...track.keyframes, { frame: rounded, value, easing: "linear" as const }]
      .sort((a, b) => a.frame - b.frame);
    const tracks = [...(animation.tracks ?? [])];
    tracks[trackIndex] = { ...track, keyframes };
    updateAnimation({ ...animation, tracks }, true);
  };

  const patchKeyframe = (
    trackIndex: number,
    kfIndex: number,
    patch: Partial<Keyframe>,
    commit: boolean,
  ) => {
    if (!animation) return;
    const tracks = [...(animation.tracks ?? [])];
    const keyframes = [...tracks[trackIndex].keyframes];
    keyframes[kfIndex] = { ...keyframes[kfIndex], ...patch };
    tracks[trackIndex] = { ...tracks[trackIndex], keyframes };
    updateAnimation({ ...animation, tracks }, commit);
  };

  const deleteKeyframe = () => {
    if (!animation || !selectedKf) return;
    const tracks = [...(animation.tracks ?? [])];
    const track = tracks[selectedKf.track];
    tracks[selectedKf.track] = {
      ...track,
      keyframes: track.keyframes.filter((_, i) => i !== selectedKf.kf),
    };
    updateAnimation({ ...animation, tracks }, true);
    setSelectedKf(null);
  };

  const onLaneMouseMove = (event: React.MouseEvent) => {
    if (scrubbingRef.current) {
      scrubTo(event);
      return;
    }
    const drag = dragKfRef.current;
    if (!drag || !animation) return;
    const frame = Math.round(frameFromEvent(event));
    if (frame !== animation.tracks?.[drag.track]?.keyframes[drag.kf]?.frame) {
      drag.moved = true;
      patchKeyframe(drag.track, drag.kf, { frame }, false);
    }
  };

  const onLaneMouseUp = () => {
    scrubbingRef.current = false;
    const drag = dragKfRef.current;
    dragKfRef.current = null;
    if (drag?.moved && animation) {
      dispatch({ type: "PUSH_HISTORY", snapshot: drag.snapshot });
      // Keep keyframes sorted after a drag.
      const tracks = [...(animation.tracks ?? [])];
      const track = tracks[drag.track];
      tracks[drag.track] = {
        ...track,
        keyframes: [...track.keyframes].sort((a, b) => a.frame - b.frame),
      };
      updateAnimation({ ...animation, tracks }, false);
    }
  };

  const selectedKeyframe =
    selectedKf && animation
      ? animation.tracks?.[selectedKf.track]?.keyframes[selectedKf.kf]
      : undefined;

  return (
    <div className="panel timeline">
      <div className="timeline-toolbar">
        <select
          value={animIndex}
          onChange={(event) => {
            setAnimIndex(Number(event.target.value));
            stop();
          }}
        >
          {animations.length === 0 && <option>— sin animaciones —</option>}
          {animations.map((anim, index) => (
            <option key={index} value={index}>{anim.name}</option>
          ))}
        </select>
        <button onClick={addAnimation} title="Nueva animación">＋</button>
        {animation && (
          <>
            <button onClick={deleteAnimation} title="Eliminar animación">🗑</button>
            <label className="tl-field">
              fps
              <input
                type="number"
                value={animation.fps}
                min={1}
                onChange={(event) =>
                  updateAnimation(
                    { ...animation, fps: Math.max(1, Number(event.target.value) || 30) },
                    true,
                  )
                }
              />
            </label>
            <label className="tl-field">
              <input
                type="checkbox"
                checked={animation.loop ?? false}
                onChange={(event) =>
                  updateAnimation({ ...animation, loop: event.target.checked }, true)
                }
              />
              loop
            </label>
            <span className="separator" />
            <button onClick={stop} title="Detener">⏹</button>
            <button
              onClick={() => {
                if (!playing && playhead >= duration) setPlayhead(0);
                setPlaying(!playing);
              }}
              title="Reproducir/Pausa"
            >
              {playing ? "⏸" : "▶"}
            </button>
            <span className="tl-frame">frame {playhead.toFixed(0)}</span>
            <span className="separator" />
            <select
              value=""
              disabled={!state.selection}
              onChange={(event) => {
                if (event.target.value) addTrack(event.target.value);
                event.target.value = "";
              }}
              title={state.selection ? "Añadir pista a la entidad seleccionada" : "Selecciona una entidad"}
            >
              <option value="">＋ pista…</option>
              {ANIMATABLE_PROPERTIES.map((property) => (
                <option key={property} value={property}>{property}</option>
              ))}
            </select>
          </>
        )}
        <span className="spacer" />
        {selectedKeyframe && selectedKf && (
          <span className="kf-editor">
            <label className="tl-field">
              frame
              <input
                type="number"
                value={selectedKeyframe.frame}
                min={0}
                onChange={(event) =>
                  patchKeyframe(selectedKf.track, selectedKf.kf, { frame: Math.max(0, Number(event.target.value) || 0) }, true)
                }
              />
            </label>
            <label className="tl-field">
              valor
              <input
                type="number"
                step="any"
                value={selectedKeyframe.value}
                onChange={(event) =>
                  patchKeyframe(selectedKf.track, selectedKf.kf, { value: Number(event.target.value) || 0 }, true)
                }
              />
            </label>
            <select
              value={selectedKeyframe.easing ?? "linear"}
              onChange={(event) =>
                patchKeyframe(selectedKf.track, selectedKf.kf, { easing: event.target.value as Keyframe["easing"] }, true)
              }
            >
              <option value="linear">linear</option>
              <option value="ease_in">ease_in</option>
              <option value="ease_out">ease_out</option>
              <option value="ease_in_out">ease_in_out</option>
            </select>
            <button onClick={deleteKeyframe} title="Eliminar keyframe">✕</button>
          </span>
        )}
      </div>

      {animation ? (
        <div className="timeline-body">
          <div className="tl-labels">
            <div className="tl-ruler-label" />
            {(animation.tracks ?? []).map((track, index) => {
              const node = findEntity(currentScene.entities, track.entity);
              return (
                <div key={index} className="tl-label">
                  <span className="tl-label-text" title={`${track.entity} · ${track.property}`}>
                    {node?.name ?? track.entity}
                    <em>.{track.property.split(".").pop()}</em>
                  </span>
                  <button onClick={() => removeTrack(index)} title="Eliminar pista">✕</button>
                </div>
              );
            })}
          </div>
          <div
            className="tl-lanes"
            ref={lanesRef}
            onMouseMove={onLaneMouseMove}
            onMouseUp={onLaneMouseUp}
            onMouseLeave={onLaneMouseUp}
          >
            <div className="tl-lanes-inner" style={{ width: maxFrames * FRAME_W }}>
              <div
                className="tl-ruler"
                onMouseDown={(event) => {
                  scrubbingRef.current = true;
                  setPlaying(false);
                  scrubTo(event);
                }}
              >
                {Array.from({ length: Math.ceil(maxFrames / RULER_STEP) }, (_, i) => (
                  <span key={i} className="tl-tick" style={{ left: i * RULER_STEP * FRAME_W }}>
                    {i * RULER_STEP}
                  </span>
                ))}
              </div>
              {(animation.tracks ?? []).map((track, trackIndex) => (
                <div
                  key={trackIndex}
                  className="tl-lane"
                  onDoubleClick={(event) => addKeyframe(trackIndex, frameFromEvent(event))}
                >
                  {track.keyframes.map((keyframe, kfIndex) => (
                    <span
                      key={kfIndex}
                      className={`tl-kf${selectedKf?.track === trackIndex && selectedKf?.kf === kfIndex ? " selected" : ""}`}
                      style={{ left: keyframe.frame * FRAME_W - 5 }}
                      title={`frame ${keyframe.frame} = ${keyframe.value}`}
                      onMouseDown={(event) => {
                        event.stopPropagation();
                        setSelectedKf({ track: trackIndex, kf: kfIndex });
                        dragKfRef.current = {
                          track: trackIndex,
                          kf: kfIndex,
                          snapshot: snapshotOf(state),
                          moved: false,
                        };
                      }}
                    />
                  ))}
                </div>
              ))}
              <div className="tl-playhead" style={{ left: playhead * FRAME_W }} />
            </div>
          </div>
        </div>
      ) : (
        <div className="panel-empty">
          Crea una animación con ＋ y añade pistas a la entidad seleccionada
        </div>
      )}
    </div>
  );
}
