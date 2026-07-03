// Document store: single source of truth for the loaded project, with
// global undo/redo over document snapshots and the editor console log.

import {
  createContext,
  useContext,
  useMemo,
  useReducer,
  type Dispatch,
  type ReactNode,
} from "react";
import type { Asset, LoadedProject, Project, Scene } from "./types";

export type LogLevel = "info" | "warn" | "error";

export interface LogEntry {
  level: LogLevel;
  message: string;
  time: string;
}

/** The undoable part of the state (everything that lands in .aigs files). */
interface DocumentSnapshot {
  project: Project;
  scenes: { path: string; scene: Scene }[];
}

export interface EditorState {
  loaded: LoadedProject | null;
  currentScenePath: string | null;
  selection: string | null;
  dirty: boolean;
  past: string[];
  future: string[];
  logs: LogEntry[];
  /** Animated view of the current scene (timeline scrub/playback); not undoable. */
  preview: Scene | null;
}

export type EditorAction =
  | { type: "PROJECT_LOADED"; loaded: LoadedProject }
  | { type: "LOG"; level: LogLevel; message: string }
  | { type: "CLEAR_LOGS" }
  | { type: "SELECT"; id: string | null }
  | { type: "SWITCH_SCENE"; path: string }
  | { type: "UPDATE_SCENE"; scene: Scene; commit: boolean }
  | { type: "UPDATE_ASSETS"; assets: Asset[] }
  | { type: "PUSH_HISTORY"; snapshot: string }
  | { type: "UNDO" }
  | { type: "REDO" }
  | { type: "SET_PREVIEW"; scene: Scene | null }
  | { type: "MARK_SAVED" };

const MAX_HISTORY = 100;

export function snapshotOf(state: EditorState): string {
  if (!state.loaded) return "";
  const document: DocumentSnapshot = {
    project: state.loaded.project,
    scenes: state.loaded.scenes,
  };
  return JSON.stringify(document);
}

function log(
  state: EditorState,
  level: LogLevel,
  message: string,
): LogEntry[] {
  const time = new Date().toLocaleTimeString();
  return [...state.logs.slice(-499), { level, message, time }];
}

function pushHistory(state: EditorState, snapshot: string): EditorState {
  return {
    ...state,
    past: [...state.past.slice(-(MAX_HISTORY - 1)), snapshot],
    future: [],
  };
}

function restore(state: EditorState, snapshot: string): EditorState {
  if (!state.loaded) return state;
  const document = JSON.parse(snapshot) as DocumentSnapshot;
  const scenePaths = document.scenes.map((s) => s.path);
  const currentScenePath =
    state.currentScenePath && scenePaths.includes(state.currentScenePath)
      ? state.currentScenePath
      : (scenePaths[0] ?? null);
  return {
    ...state,
    loaded: {
      ...state.loaded,
      project: document.project,
      scenes: document.scenes,
    },
    currentScenePath,
    dirty: true,
  };
}

export function reducer(
  state: EditorState,
  action: EditorAction,
): EditorState {
  switch (action.type) {
    case "PROJECT_LOADED":
      return {
        ...state,
        loaded: action.loaded,
        currentScenePath:
          action.loaded.scenes.find(
            (s) => s.path === action.loaded.project.initial_scene,
          )?.path ??
          action.loaded.scenes[0]?.path ??
          null,
        selection: null,
        dirty: false,
        past: [],
        future: [],
        preview: null,
        logs: log(
          state,
          "info",
          `Proyecto "${action.loaded.project.name}" cargado (${action.loaded.scenes.length} escena(s), ${action.loaded.project.assets.length} asset(s))`,
        ),
      };
    case "LOG":
      return { ...state, logs: log(state, action.level, action.message) };
    case "CLEAR_LOGS":
      return { ...state, logs: [] };
    case "SELECT":
      return { ...state, selection: action.id };
    case "SWITCH_SCENE":
      return {
        ...state,
        currentScenePath: action.path,
        selection: null,
        preview: null,
      };
    case "SET_PREVIEW":
      return { ...state, preview: action.scene };
    case "UPDATE_SCENE": {
      if (!state.loaded || !state.currentScenePath) return state;
      const base = action.commit ? pushHistory(state, snapshotOf(state)) : state;
      return {
        ...base,
        dirty: true,
        loaded: {
          ...base.loaded!,
          scenes: base.loaded!.scenes.map((entry) =>
            entry.path === state.currentScenePath
              ? { ...entry, scene: action.scene }
              : entry,
          ),
        },
      };
    }
    case "UPDATE_ASSETS": {
      if (!state.loaded) return state;
      const base = pushHistory(state, snapshotOf(state));
      return {
        ...base,
        dirty: true,
        loaded: {
          ...base.loaded!,
          project: { ...base.loaded!.project, assets: action.assets },
        },
      };
    }
    case "PUSH_HISTORY":
      return pushHistory(state, action.snapshot);
    case "UNDO": {
      const snapshot = state.past[state.past.length - 1];
      if (snapshot === undefined) return state;
      return {
        ...restore(state, snapshot),
        past: state.past.slice(0, -1),
        future: [...state.future, snapshotOf(state)],
      };
    }
    case "REDO": {
      const snapshot = state.future[state.future.length - 1];
      if (snapshot === undefined) return state;
      return {
        ...restore(state, snapshot),
        past: [...state.past, snapshotOf(state)],
        future: state.future.slice(0, -1),
      };
    }
    case "MARK_SAVED":
      return {
        ...state,
        dirty: false,
        logs: log(state, "info", "Proyecto guardado"),
      };
    default:
      return state;
  }
}

export const initialState: EditorState = {
  loaded: null,
  currentScenePath: null,
  selection: null,
  dirty: false,
  past: [],
  future: [],
  logs: [],
  preview: null,
};

interface StoreValue {
  state: EditorState;
  dispatch: Dispatch<EditorAction>;
  currentScene: Scene | null;
}

const StoreContext = createContext<StoreValue | null>(null);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const currentScene = useMemo(
    () =>
      state.loaded?.scenes.find((s) => s.path === state.currentScenePath)
        ?.scene ?? null,
    [state.loaded, state.currentScenePath],
  );
  const value = useMemo(
    () => ({ state, dispatch, currentScene }),
    [state, currentScene],
  );
  return (
    <StoreContext.Provider value={value}>{children}</StoreContext.Provider>
  );
}

export function useStore(): StoreValue {
  const value = useContext(StoreContext);
  if (!value) throw new Error("useStore outside StoreProvider");
  return value;
}
