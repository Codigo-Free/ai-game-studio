import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  loadProject,
  createProject,
  exportProject,
  playProject,
  saveProject,
  type ExportTarget,
} from "../ipc";
import { useStore } from "../store";
import type { Scene } from "../types";

const EXPORT_TARGET_LABEL: Record<ExportTarget, string> = {
  desktop: "Escritorio",
  web: "Web",
  android: "Android",
};

function sceneSlug(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "escena"
  );
}

function emptyScene(name: string): Scene {
  return {
    format: { kind: "aigs-scene", version: 0 },
    name,
    entities: [
      {
        id: "camera",
        name: "Main Camera",
        components: { transform2d: {}, camera2d: { zoom: 1 } },
      },
    ],
    animations: [],
  };
}

export function useProjectActions() {
  const { state, dispatch } = useStore();

  const openProject = async () => {
    try {
      const selected = await open({
        title: "Abrir proyecto AI Game Studio",
        filters: [{ name: "Proyecto AIGS", extensions: ["aigs"] }],
        multiple: false,
      });
      if (typeof selected !== "string") return;
      dispatch({
        type: "PROJECT_LOADED",
        loaded: await loadProject(selected),
      });
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
    }
  };

  const newProject = async () => {
    try {
      const directory = await open({
        title: "Carpeta para el nuevo proyecto",
        directory: true,
      });
      if (typeof directory !== "string") return;
      const name = window.prompt("Nombre del juego", "Mi Juego");
      if (!name) return;
      dispatch({
        type: "PROJECT_LOADED",
        loaded: await createProject(directory, name),
      });
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
    }
  };

  const save = async () => {
    if (!state.loaded) return false;
    try {
      await saveProject(
        state.loaded.manifest_path,
        state.loaded.project,
        state.loaded.scenes,
      );
      dispatch({ type: "MARK_SAVED" });
      return true;
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
      return false;
    }
  };

  const play = async () => {
    if (!state.loaded) return;
    if (!(await save())) return;
    try {
      const message = await playProject(state.loaded.manifest_path);
      dispatch({ type: "LOG", level: "info", message });
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
    }
  };

  const exportGame = async (target: ExportTarget) => {
    if (!state.loaded) return;
    if (!(await save())) return;
    try {
      const directory = await open({
        title: `Carpeta de destino de la exportación (${EXPORT_TARGET_LABEL[target]})`,
        directory: true,
      });
      if (typeof directory !== "string") return;
      const message = await exportProject(
        state.loaded.manifest_path,
        directory,
        target,
      );
      for (const line of message.split("\n")) {
        dispatch({ type: "LOG", level: "info", message: `[export] ${line}` });
      }
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
    }
  };

  return { openProject, newProject, save, play, exportGame };
}

function useSceneActions() {
  const { state, dispatch, currentScene } = useStore();
  const loaded = state.loaded;

  const uniquePath = (base: string): string => {
    const taken = new Set(loaded?.project.scenes ?? []);
    let path = `scenes/${base}.scene.aigs`;
    let counter = 2;
    while (taken.has(path)) {
      path = `scenes/${base}-${counter}.scene.aigs`;
      counter += 1;
    }
    return path;
  };

  const addScene = (name: string, scene: Scene) => {
    if (!loaded) return;
    const path = uniquePath(sceneSlug(name));
    dispatch({
      type: "UPDATE_DOCUMENT",
      project: {
        ...loaded.project,
        scenes: [...loaded.project.scenes, path],
      },
      scenes: [...loaded.scenes, { path, scene: { ...scene, name } }],
      switchTo: path,
      commit: true,
    });
    dispatch({ type: "LOG", level: "info", message: `Escena "${name}" creada (${path})` });
  };

  const newScene = () => {
    const name = window.prompt("Nombre de la nueva escena", "nivel");
    if (name) addScene(name, emptyScene(name));
  };

  const duplicateScene = () => {
    if (!currentScene) return;
    const name = window.prompt("Nombre de la copia", `${currentScene.name}-copia`);
    if (name) addScene(name, structuredClone(currentScene));
  };

  const deleteScene = () => {
    if (!loaded || !state.currentScenePath) return;
    if (loaded.scenes.length <= 1) {
      dispatch({ type: "LOG", level: "warn", message: "No se puede eliminar la única escena" });
      return;
    }
    if (!window.confirm(`¿Eliminar la escena "${currentScene?.name}"?`)) return;
    const path = state.currentScenePath;
    const scenes = loaded.scenes.filter((entry) => entry.path !== path);
    const scenePaths = loaded.project.scenes.filter((p) => p !== path);
    const initial =
      loaded.project.initial_scene === path
        ? scenePaths[0]
        : loaded.project.initial_scene;
    dispatch({
      type: "UPDATE_DOCUMENT",
      project: { ...loaded.project, scenes: scenePaths, initial_scene: initial },
      scenes,
      commit: true,
    });
    dispatch({ type: "LOG", level: "info", message: `Escena eliminada (${path})` });
  };

  const setInitialScene = () => {
    if (!loaded || !state.currentScenePath) return;
    dispatch({
      type: "UPDATE_DOCUMENT",
      project: { ...loaded.project, initial_scene: state.currentScenePath },
      scenes: loaded.scenes,
      switchTo: state.currentScenePath,
      commit: true,
    });
  };

  return { newScene, duplicateScene, deleteScene, setInitialScene };
}

export function Toolbar() {
  const { state, dispatch } = useStore();
  const { openProject, newProject, save, play, exportGame } =
    useProjectActions();
  const { newScene, duplicateScene, deleteScene, setInitialScene } =
    useSceneActions();
  const loaded = state.loaded;
  const [exportTarget, setExportTarget] = useState<ExportTarget>("desktop");

  return (
    <header className="toolbar">
      <span className="brand">AI Game Studio</span>
      <button onClick={newProject}>Nuevo</button>
      <button onClick={openProject}>Abrir…</button>
      <button onClick={save} disabled={!loaded}>
        Guardar{state.dirty ? " •" : ""}
      </button>
      <span className="separator" />
      <button
        onClick={() => dispatch({ type: "UNDO" })}
        disabled={state.past.length === 0}
        title="Deshacer (Ctrl+Z)"
      >
        ⤺
      </button>
      <button
        onClick={() => dispatch({ type: "REDO" })}
        disabled={state.future.length === 0}
        title="Rehacer (Ctrl+Shift+Z)"
      >
        ⤻
      </button>
      <span className="separator" />
      {loaded && loaded.scenes.length > 0 && (
        <>
          <select
            value={state.currentScenePath ?? ""}
            onChange={(event) =>
              dispatch({ type: "SWITCH_SCENE", path: event.target.value })
            }
          >
            {loaded.scenes.map((entry) => (
              <option key={entry.path} value={entry.path}>
                {entry.path === loaded.project.initial_scene ? "★ " : ""}
                {entry.scene.name}
              </option>
            ))}
          </select>
          <button onClick={newScene} title="Nueva escena">＋</button>
          <button onClick={duplicateScene} title="Duplicar escena">⧉</button>
          <button
            onClick={setInitialScene}
            title="Marcar como escena inicial"
            disabled={state.currentScenePath === loaded.project.initial_scene}
          >
            ★
          </button>
          <button onClick={deleteScene} title="Eliminar escena">✕</button>
        </>
      )}
      <span className="spacer" />
      {loaded && <span className="project-name">{loaded.project.name}</span>}
      <select
        value={exportTarget}
        onChange={(event) =>
          setExportTarget(event.target.value as ExportTarget)
        }
        title="Plataforma de exportación"
      >
        {(Object.keys(EXPORT_TARGET_LABEL) as ExportTarget[]).map((target) => (
          <option key={target} value={target}>
            {EXPORT_TARGET_LABEL[target]}
          </option>
        ))}
      </select>
      <button
        onClick={() => exportGame(exportTarget)}
        disabled={!loaded}
        title={`Exportar a ${EXPORT_TARGET_LABEL[exportTarget]}`}
      >
        ⬇ Exportar
      </button>
      <button className="play" onClick={play} disabled={!loaded}>
        ▶ Play
      </button>
    </header>
  );
}
