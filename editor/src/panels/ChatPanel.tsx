import { useEffect, useRef, useState } from "react";
import {
  allEntityIds,
  findEntity,
  insertEntity,
  mergeComponentsPatch,
  patchComponents,
  removeEntity,
} from "../document";
import { askAi, proposeChange, writeScriptAsset } from "../ipc";
import { useStore } from "../store";
import type { Asset, ChangeProposal, Project, Scene } from "../types";

// Cap on how much of the project we hand to the model — a crude but
// simple guard against blowing past a smaller local model's context
// window on a large project (milestone M18; no smarter summarization
// yet, see docs/plan.md).
const MAX_CONTEXT_CHARS = 12000;

type ChatMode = "ask" | "propose";

type ChatEntry =
  | { role: "user" | "assistant" | "error"; text: string }
  | {
      role: "proposal";
      proposal: ChangeProposal;
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
      } else {
        const knownAssets = state.loaded.project.assets.map((a) => ({
          id: a.id,
          kind: a.kind,
        }));
        const knownEntityIds = allEntityIds(currentScene?.entities ?? []);
        const proposal = await proposeChange(
          context,
          trimmed,
          knownAssets,
          knownEntityIds,
        );
        setHistory((h) => [...h, { role: "proposal", proposal, status: "pending" }]);
      }
    } catch (error) {
      setHistory((h) => [...h, { role: "error", text: String(error) }]);
    } finally {
      setAsking(false);
    }
  };

  const discardProposal = (index: number) => {
    setHistory((h) =>
      h.map((entry, i) =>
        i === index && entry.role === "proposal"
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

      let entities = currentScene.entities;
      for (const add of proposal.entities_to_add) {
        entities = insertEntity(entities, add.parent_id, add.entity);
      }
      for (const update of proposal.entities_to_update) {
        const node = findEntity(entities, update.id);
        if (!node) continue;
        const merged = mergeComponentsPatch(
          node.components ?? {},
          update.components_patch,
        );
        entities = patchComponents(entities, update.id, merged);
      }
      for (const id of proposal.entities_to_remove) {
        entities = removeEntity(entities, id);
      }

      const newScene: Scene = { ...currentScene, entities };
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
            {mode === "ask"
              ? 'Pregunta algo sobre el proyecto abierto, por ejemplo "¿qué comportamientos tiene la entidad robot?".'
              : 'Describe un cambio y revísalo antes de aplicarlo, por ejemplo "añade un enemigo que patrulle entre estos dos puntos".'}
          </p>
        )}
        {history.map((entry, index) => {
          if (entry.role === "proposal") {
            const { proposal, status } = entry;
            return (
              <div key={index} className="chat-entry chat-proposal">
                <div className="proposal-summary">{proposal.summary}</div>
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
                    <li key={`script-${script.asset_id}`}>
                      📜 Script nuevo "{script.filename}"
                    </li>
                  ))}
                </ul>
                {status === "pending" ? (
                  <div className="proposal-actions">
                    <button
                      onClick={() => void applyProposal(index, proposal)}
                      disabled={busyIndex !== null}
                    >
                      Aplicar
                    </button>
                    <button
                      onClick={() => discardProposal(index)}
                      disabled={busyIndex !== null}
                    >
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
              : "Describe el cambio que quieres…"
          }
          disabled={!state.loaded || asking}
        />
        <button type="submit" disabled={!state.loaded || asking}>
          {mode === "ask" ? "Enviar" : "Proponer"}
        </button>
      </form>
    </div>
  );
}
