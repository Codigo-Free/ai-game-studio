import { open } from "@tauri-apps/plugin-dialog";
import { loadProject, createProject, playProject, saveProject } from "../ipc";
import { useStore } from "../store";

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

  return { openProject, newProject, save, play };
}

export function Toolbar() {
  const { state, dispatch } = useStore();
  const { openProject, newProject, save, play } = useProjectActions();
  const loaded = state.loaded;

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
        <select
          value={state.currentScenePath ?? ""}
          onChange={(event) =>
            dispatch({ type: "SWITCH_SCENE", path: event.target.value })
          }
        >
          {loaded.scenes.map((entry) => (
            <option key={entry.path} value={entry.path}>
              {entry.scene.name}
            </option>
          ))}
        </select>
      )}
      <span className="spacer" />
      {loaded && <span className="project-name">{loaded.project.name}</span>}
      <button className="play" onClick={play} disabled={!loaded}>
        ▶ Play
      </button>
    </header>
  );
}
