import { useEffect, useRef, useState } from "react";
import {
  allEntityIds,
  findEntity,
  insertEntity,
  mergeComponentsPatch,
  patchComponents,
  removeEntity,
} from "../document";
import {
  askAi,
  generateProject,
  orchestrateChange,
  proposeChange,
  writeScriptAsset,
} from "../ipc";
import { useStore } from "../store";
import type {
  Asset,
  ChangeProposal,
  EntityNode,
  Project,
  ProjectProposal,
  Scene,
} from "../types";

// Cap on how much of the project we hand to the model — a crude but
// simple guard against blowing past a smaller local model's context
// window on a large project (milestone M18; no smarter summarization
// yet, see docs/plan.md).
const MAX_CONTEXT_CHARS = 12000;

type ChatMode = "ask" | "propose" | "orchestrate" | "generate";

type ChatEntry =
  | { role: "user" | "assistant" | "error"; text: string }
  | {
      role: "proposal";
      proposal: ChangeProposal;
      status: "pending" | "applied" | "discarded";
    }
  | {
      role: "project-proposal";
      project: ProjectProposal;
      status: "pending" | "applied" | "discarded";
    };

function buildContext(
  project: { name: string; description?: string; scenes: string[] } | null,
  currentScenePath: string | null,
  currentScene: unknown,
): string {
  if (!project) return "No project is open.";
  const summary = {
    project: {
      name: project.name,
      description: project.description ?? null,
      scenes: project.scenes,
    },
    current_scene_path: currentScenePath,
    current_scene: currentScene,
  };
  const text = JSON.stringify(summary, null, 2);
  return text.length > MAX_CONTEXT_CHARS
    ? `${text.slice(0, MAX_CONTEXT_CHARS)}\n… (truncated, project too large for the current context budget)`
    : text;
}

/** Turns a display name into a filesystem-safe scene slug, falling back to
 * "scene" for a name with no alphanumeric characters at all. */
function slugifyName(name: string): string {
  const base = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return base || "scene";
}

/** Picks a `scenes/<slug>.scene.aigs` path for a new scene, avoiding any
 * path already in the project or already allocated earlier in this apply. */
function uniqueScenePath(name: string, existingPaths: string[]): string {
  const base = slugifyName(name);
  let path = `scenes/${base}.scene.aigs`;
  let counter = 2;
  while (existingPaths.includes(path)) {
    path = `scenes/${base}-${counter}.scene.aigs`;
    counter += 1;
  }
  return path;
}

/** Applies a `ChangeProposal`'s entity/scene-level changes onto an entity
 * list, returning the updated list (pure — used for both the currently
 * open scene and a freshly-scaffolded new one). */
function applyEntityChanges(
  entities: EntityNode[],
  proposal: ChangeProposal,
): EntityNode[] {
  let next = entities;
  for (const add of proposal.entities_to_add) {
    next = insertEntity(next, add.parent_id, add.entity);
  }
  for (const update of proposal.entities_to_update) {
    const node = findEntity(next, update.id);
    if (!node) continue;
    const merged = mergeComponentsPatch(node.components ?? {}, update.components_patch);
    next = patchComponents(next, update.id, merged);
  }
  for (const id of proposal.entities_to_remove) {
    next = removeEntity(next, id);
  }
  return next;
}

function ProposalDetails({ proposal }: { proposal: ChangeProposal }) {
  return (
    <ul className="proposal-details">
      {proposal.entities_to_add.map((add) => (
        <li key={`add-${add.entity.id}`}>
          + Entidad "{add.entity.name}" ({add.entity.id})
        </li>
      ))}
      {proposal.entities_to_update.map((update) => (
        <li key={`upd-${update.id}`}>~ Actualiza "{update.id}"</li>
      ))}
      {proposal.entities_to_remove.map((id) => (
        <li key={`rem-${id}`}>− Elimina "{id}"</li>
      ))}
      {proposal.scripts.map((script) => (
        <li key={`script-${script.asset_id}`}>📜 Script nuevo "{script.filename}"</li>
      ))}
      {proposal.scene_patch.gravity && (
        <li key="scene-gravity">
          ~ Gravedad de la escena → ({proposal.scene_patch.gravity.x},{" "}
          {proposal.scene_patch.gravity.y})
        </li>
      )}
      {proposal.scene_patch.music && (
        <li key="scene-music">
          🎵 Música de la escena → "{proposal.scene_patch.music.asset}"
        </li>
      )}
    </ul>
  );
}

export function ChatPanel() {
  const { state, dispatch, currentScene } = useStore();
  const [mode, setMode] = useState<ChatMode>("ask");
  const [history, setHistory] = useState<ChatEntry[]>([]);
  const [question, setQuestion] = useState("");
  const [asking, setAsking] = useState(false);
  const [busyIndex, setBusyIndex] = useState<number | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const body = bodyRef.current;
    if (body) body.scrollTop = body.scrollHeight;
  }, [history]);

  const submit = async () => {
    const trimmed = question.trim();
    if (!trimmed || asking || !state.loaded) return;
    setHistory((h) => [...h, { role: "user", text: trimmed }]);
    setQuestion("");
    setAsking(true);
    try {
      const context = buildContext(
        state.loaded.project,
        state.currentScenePath,
        currentScene,
      );
      if (mode === "ask") {
        const answer = await askAi(context, trimmed);
        setHistory((h) => [...h, { role: "assistant", text: answer }]);
      } else if (mode === "generate") {
        const knownAssets = state.loaded.project.assets.map((a) => ({
          id: a.id,
          kind: a.kind,
        }));
        const knownSceneNames = state.loaded.scenes.map((entry) => entry.scene.name);
        const currentEntityIds = allEntityIds(currentScene?.entities ?? []);
        const currentAnimationNames = (currentScene?.animations ?? []).map(
          (a) => a.name,
        );
        const project = await generateProject(
          context,
          trimmed,
          knownAssets,
          knownSceneNames,
          currentEntityIds,
          currentAnimationNames,
        );
        setHistory((h) => [
          ...h,
          { role: "project-proposal", project, status: "pending" },
        ]);
      } else {
        const knownAssets = state.loaded.project.assets.map((a) => ({
          id: a.id,
          kind: a.kind,
        }));
        const knownEntityIds = allEntityIds(currentScene?.entities ?? []);
        const knownAnimationNames = (currentScene?.animations ?? []).map(
          (a) => a.name,
        );
        const proposal = await (mode === "orchestrate"
          ? orchestrateChange(
              context,
              trimmed,
              knownAssets,
              knownEntityIds,
              knownAnimationNames,
            )
          : proposeChange(
              context,
              trimmed,
              knownAssets,
              knownEntityIds,
              knownAnimationNames,
            ));
        setHistory((h) => [...h, { role: "proposal", proposal, status: "pending" }]);
      }
    } catch (error) {
      setHistory((h) => [...h, { role: "error", text: String(error) }]);
    } finally {
      setAsking(false);
    }
  };

  const discard = (index: number) => {
    setHistory((h) =>
      h.map((entry, i) =>
        i === index && (entry.role === "proposal" || entry.role === "project-proposal")
          ? { ...entry, status: "discarded" }
          : entry,
      ),
    );
  };

  const applyProposal = async (index: number, proposal: ChangeProposal) => {
    if (!state.loaded || !currentScene || !state.currentScenePath) return;
    setBusyIndex(index);
    try {
      const newScriptAssets: Asset[] = [];
      for (const script of proposal.scripts) {
        const written = await writeScriptAsset(
          state.loaded.root,
          script.asset_id,
          script.filename,
          script.content,
        );
        newScriptAssets.push({ id: written.id, kind: "script", path: written.path });
      }

      const entities = applyEntityChanges(currentScene.entities, proposal);
      const newScene: Scene = {
        ...currentScene,
        entities,
        gravity: proposal.scene_patch.gravity ?? currentScene.gravity,
        music: proposal.scene_patch.music ?? currentScene.music,
      };
      const newProject: Project = {
        ...state.loaded.project,
        assets: [...state.loaded.project.assets, ...newScriptAssets],
      };
      const newScenes = state.loaded.scenes.map((entry) =>
        entry.path === state.currentScenePath
          ? { ...entry, scene: newScene }
          : entry,
      );
      dispatch({
        type: "UPDATE_DOCUMENT",
        project: newProject,
        scenes: newScenes,
        commit: true,
      });
      dispatch({
        type: "LOG",
        level: "info",
        message: `IA aplicó cambios: ${proposal.summary}`,
      });
      setHistory((h) =>
        h.map((entry, i) =>
          i === index && entry.role === "proposal"
            ? { ...entry, status: "applied" }
            : entry,
        ),
      );
    } catch (error) {
      dispatch({
        type: "LOG",
        level: "error",
        message: `No se pudo aplicar la propuesta de la IA: ${error}`,
      });
    } finally {
      setBusyIndex(null);
    }
  };

  const applyProjectProposal = async (index: number, project: ProjectProposal) => {
    if (!state.loaded || !currentScene || !state.currentScenePath) return;
    setBusyIndex(index);
    try {
      let assets = [...state.loaded.project.assets];
      let currentEntities = currentScene.entities;
      let currentGravity = currentScene.gravity;
      let currentMusic = currentScene.music;
      const newScenes: { path: string; scene: Scene }[] = [];
      const allocatedPaths = state.loaded.scenes.map((entry) => entry.path);

      for (const scened of project.scenes) {
        const { proposal } = scened;
        for (const script of proposal.scripts) {
          const written = await writeScriptAsset(
            state.loaded.root,
            script.asset_id,
            script.filename,
            script.content,
          );
          assets = [...assets, { id: written.id, kind: "script", path: written.path }];
        }

        if (scened.is_new) {
          const entities = applyEntityChanges([], proposal);
          const path = uniqueScenePath(scened.name, allocatedPaths);
          allocatedPaths.push(path);
          newScenes.push({
            path,
            scene: {
              format: { kind: "aigs-scene", version: 0 },
              name: scened.name,
              entities,
              animations: [],
              gravity: proposal.scene_patch.gravity,
              music: proposal.scene_patch.music,
            },
          });
        } else {
          currentEntities = applyEntityChanges(currentEntities, proposal);
          currentGravity = proposal.scene_patch.gravity ?? currentGravity;
          currentMusic = proposal.scene_patch.music ?? currentMusic;
        }
      }

      const updatedCurrentScene: Scene = {
        ...currentScene,
        entities: currentEntities,
        gravity: currentGravity,
        music: currentMusic,
      };
      const allScenes = state.loaded.scenes.map((entry) =>
        entry.path === state.currentScenePath
          ? { ...entry, scene: updatedCurrentScene }
          : entry,
      );
      const finalScenes = [...allScenes, ...newScenes];
      const newProject: Project = {
        ...state.loaded.project,
        assets,
        scenes: [...state.loaded.project.scenes, ...newScenes.map((s) => s.path)],
      };
      dispatch({
        type: "UPDATE_DOCUMENT",
        project: newProject,
        scenes: finalScenes,
        commit: true,
      });
      dispatch({
        type: "LOG",
        level: "info",
        message: `IA generó el juego: ${project.summary.split("\n")[0]}`,
      });
      setHistory((h) =>
        h.map((entry, i) =>
          i === index && entry.role === "project-proposal"
            ? { ...entry, status: "applied" }
            : entry,
        ),
      );
    } catch (error) {
      dispatch({
        type: "LOG",
        level: "error",
        message: `No se pudo aplicar la generación de la IA: ${error}`,
      });
    } finally {
      setBusyIndex(null);
    }
  };

  return (
    <div className="panel chat">
      <div className="panel-header chat-header">
        <div className="chat-modes">
          <button
            className={mode === "ask" ? "active" : ""}
            onClick={() => setMode("ask")}
          >
            Preguntar
          </button>
          <button
            className={mode === "propose" ? "active" : ""}
            onClick={() => setMode("propose")}
          >
            Proponer cambios
          </button>
          <button
            className={mode === "orchestrate" ? "active" : ""}
            onClick={() => setMode("orchestrate")}
          >
            Orquestar agentes
          </button>
          <button
            className={mode === "generate" ? "active" : ""}
            onClick={() => setMode("generate")}
          >
            Generar juego
          </button>
        </div>
        <button
          className="panel-header-action"
          onClick={() => setHistory([])}
          disabled={history.length === 0}
        >
          Limpiar
        </button>
      </div>
      <div className="panel-body chat-body" ref={bodyRef}>
        {history.length === 0 && (
          <p className="chat-hint">
            {mode === "ask" &&
              'Pregunta algo sobre el proyecto abierto, por ejemplo "¿qué comportamientos tiene la entidad robot?".'}
            {mode === "propose" &&
              'Describe un cambio y revísalo antes de aplicarlo, por ejemplo "añade un enemigo que patrulle entre estos dos puntos".'}
            {mode === "orchestrate" &&
              'Describe algo de más alcance: un Arquitecto lo repartirá entre especialistas (estructura, física, audio…), por ejemplo "crea una plataforma con colisión y ponle música de fondo".'}
            {mode === "generate" &&
              'Describe el juego que quieres: un Productor decidirá qué escenas hacen falta (nuevas o la ya abierta), por ejemplo "un menú con título y un nivel con una plataforma". Necesita que ya hayas importado los sprites/audio que quieras usar.'}
          </p>
        )}
        {history.map((entry, index) => {
          if (entry.role === "proposal") {
            const { proposal, status } = entry;
            return (
              <div key={index} className="chat-entry chat-proposal">
                <div className="proposal-summary">{proposal.summary}</div>
                <ProposalDetails proposal={proposal} />
                {status === "pending" ? (
                  <div className="proposal-actions">
                    <button
                      onClick={() => void applyProposal(index, proposal)}
                      disabled={busyIndex !== null}
                    >
                      Aplicar
                    </button>
                    <button onClick={() => discard(index)} disabled={busyIndex !== null}>
                      Descartar
                    </button>
                  </div>
                ) : (
                  <div className="proposal-status">
                    {status === "applied" ? "✓ Aplicado" : "Descartado"}
                  </div>
                )}
              </div>
            );
          }
          if (entry.role === "project-proposal") {
            const { project, status } = entry;
            return (
              <div key={index} className="chat-entry chat-proposal chat-project-proposal">
                <div className="proposal-summary">{project.summary}</div>
                {project.scenes.map((scened, sceneIndex) => (
                  <div key={sceneIndex} className="project-scene">
                    <div className="project-scene-title">
                      {scened.is_new
                        ? `Escena nueva "${scened.name}"`
                        : `Escena actual ("${scened.name}")`}
                    </div>
                    <ProposalDetails proposal={scened.proposal} />
                  </div>
                ))}
                {status === "pending" ? (
                  <div className="proposal-actions">
                    <button
                      onClick={() => void applyProjectProposal(index, project)}
                      disabled={busyIndex !== null}
                    >
                      Aplicar
                    </button>
                    <button onClick={() => discard(index)} disabled={busyIndex !== null}>
                      Descartar
                    </button>
                  </div>
                ) : (
                  <div className="proposal-status">
                    {status === "applied" ? "✓ Aplicado" : "Descartado"}
                  </div>
                )}
              </div>
            );
          }
          return (
            <div key={index} className={`chat-entry chat-${entry.role}`}>
              {entry.text}
            </div>
          );
        })}
        {asking && <div className="chat-entry chat-assistant chat-pending">…</div>}
      </div>
      <form
        className="chat-input"
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <input
          type="text"
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          placeholder={
            mode === "ask"
              ? "Pregunta sobre el proyecto…"
              : mode === "propose"
                ? "Describe el cambio que quieres…"
                : mode === "orchestrate"
                  ? "Describe el objetivo de alto nivel…"
                  : "Describe el juego que quieres…"
          }
          disabled={!state.loaded || asking}
        />
        <button type="submit" disabled={!state.loaded || asking}>
          {mode === "ask"
            ? "Enviar"
            : mode === "propose"
              ? "Proponer"
              : mode === "orchestrate"
                ? "Orquestar"
                : "Generar"}
        </button>
      </form>
    </div>
  );
}
