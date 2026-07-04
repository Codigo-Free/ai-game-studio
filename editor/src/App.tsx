import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { removeEntity } from "./document";
import { AssetsPanel } from "./panels/AssetsPanel";
import { ConsolePanel } from "./panels/ConsolePanel";
import { Inspector } from "./panels/Inspector";
import { SceneTree } from "./panels/SceneTree";
import { Timeline } from "./panels/Timeline";
import { Toolbar, useProjectActions } from "./panels/Toolbar";
import { Viewport } from "./panels/Viewport";
import { StoreProvider, useStore } from "./store";
import "./App.css";

function Shortcuts() {
  const { state, dispatch, currentScene } = useStore();
  const { save } = useProjectActions();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const typing =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT";
      if (event.ctrlKey || event.metaKey) {
        const key = event.key.toLowerCase();
        if (key === "z" && !event.shiftKey) {
          event.preventDefault();
          dispatch({ type: "UNDO" });
        } else if (key === "y" || (key === "z" && event.shiftKey)) {
          event.preventDefault();
          dispatch({ type: "REDO" });
        } else if (key === "s") {
          event.preventDefault();
          void save();
        }
        return;
      }
      if (event.key === "Delete" && !typing && state.selection && currentScene) {
        dispatch({
          type: "UPDATE_SCENE",
          scene: {
            ...currentScene,
            entities: removeEntity(currentScene.entities, state.selection),
          },
          commit: true,
        });
        dispatch({ type: "SELECT", id: null });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [state.selection, currentScene, dispatch, save]);

  return null;
}

function Welcome() {
  const { openProject, newProject } = useProjectActions();
  return (
    <div className="welcome">
      <h1>AI Game Studio</h1>
      <p>Build Games at the Speed of Imagination</p>
      <div className="welcome-actions">
        <button onClick={newProject}>Nuevo proyecto…</button>
        <button onClick={openProject}>Abrir proyecto…</button>
      </div>
      <p className="hint">
        Abre el <code>game.aigs</code> de un proyecto, por ejemplo{" "}
        <code>examples/hello-world/game.aigs</code>
      </p>
    </div>
  );
}

/** Streams the player's stdout/stderr (aigs run) into the editor console. */
function usePlayerLogs() {
  const { dispatch } = useStore();
  useEffect(() => {
    const unlistenLog = listen<string>("player-log", (event) => {
      dispatch({
        type: "LOG",
        level: "info",
        message: `[player] ${event.payload}`,
      });
    });
    const unlistenErr = listen<string>("player-err", (event) => {
      dispatch({
        type: "LOG",
        level: "warn",
        message: `[player] ${event.payload}`,
      });
    });
    return () => {
      void unlistenLog.then((f) => f());
      void unlistenErr.then((f) => f());
    };
  }, [dispatch]);
}

function Layout() {
  const { state } = useStore();
  const [bottomTab, setBottomTab] = useState<"timeline" | "console">(
    "timeline",
  );
  usePlayerLogs();
  if (!state.loaded) {
    return (
      <div className="app">
        <Toolbar />
        <Welcome />
        <ConsolePanel />
      </div>
    );
  }
  return (
    <div className="app">
      <Toolbar />
      <Shortcuts />
      <div className="workspace">
        <div className="left-column">
          <SceneTree />
          <AssetsPanel />
        </div>
        <Viewport />
        <Inspector />
      </div>
      <div className="bottom-area">
        <div className="bottom-tabs">
          <button
            className={bottomTab === "timeline" ? "active" : ""}
            onClick={() => setBottomTab("timeline")}
          >
            Timeline
          </button>
          <button
            className={bottomTab === "console" ? "active" : ""}
            onClick={() => setBottomTab("console")}
          >
            Consola
          </button>
        </div>
        {bottomTab === "timeline" ? <Timeline /> : <ConsolePanel />}
      </div>
    </div>
  );
}

export default function App() {
  return (
    <StoreProvider>
      <Layout />
    </StoreProvider>
  );
}
