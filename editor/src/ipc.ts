// Bridge to the Tauri backend (editor/src-tauri/src/lib.rs).

import { invoke } from "@tauri-apps/api/core";
import type {
  ChangeProposal,
  LoadedProject,
  Project,
  ProjectProposal,
  Scene,
} from "./types";

export function loadProject(manifestPath: string): Promise<LoadedProject> {
  return invoke("load_project", { manifestPath });
}

export function createProject(
  directory: string,
  name: string,
): Promise<LoadedProject> {
  return invoke("create_project", { directory, name });
}

export function saveProject(
  manifestPath: string,
  project: Project,
  scenes: { path: string; scene: Scene }[],
): Promise<void> {
  return invoke("save_project", {
    manifestPath,
    projectJson: JSON.stringify(project),
    scenes: scenes.map(({ path, scene }) => [path, JSON.stringify(scene)]),
  });
}

export function importAsset(
  projectRoot: string,
  sourcePath: string,
): Promise<{ id: string; path: string }> {
  return invoke("import_asset", { projectRoot, sourcePath });
}

export function readFileBase64(path: string): Promise<string> {
  return invoke("read_file_base64", { path });
}

export function playProject(manifestPath: string): Promise<string> {
  return invoke("play_project", { manifestPath });
}

export type ExportTarget = "desktop" | "web" | "android";

export function exportProject(
  manifestPath: string,
  outputDir: string,
  target: ExportTarget,
): Promise<string> {
  return invoke("export_project", { manifestPath, outputDir, target });
}

/** Asks the AI Core a question about `context` (a summary of the project
 * currently open in the editor, built by the caller — see ChatPanel). */
export function askAi(context: string, question: string): Promise<string> {
  return invoke("ai_chat", { context, question });
}

/** Asks the AI Core to propose a change to the current scene (milestone
 * M19). `knownAssets`/`knownEntityIds`/`knownAnimationNames` let the
 * backend reject a proposal that references something that doesn't exist,
 * before it's ever shown to the user. Rejects (does not resolve to a
 * partial proposal) if the model's answer fails validation. */
export function proposeChange(
  context: string,
  instruction: string,
  knownAssets: { id: string; kind: string }[],
  knownEntityIds: string[],
  knownAnimationNames: string[],
): Promise<ChangeProposal> {
  return invoke("ai_propose_change", {
    context,
    instruction,
    knownAssets,
    knownEntityIds,
    knownAnimationNames,
  });
}

/** Asks the AI Core to fulfill a high-level instruction by coordinating
 * several specialized agents (milestone M20: an Architect plans steps,
 * each step runs through its own scoped specialist). Returns the same
 * `ChangeProposal` shape as `proposeChange`, so callers can reuse the same
 * review/apply UI for both. */
export function orchestrateChange(
  context: string,
  instruction: string,
  knownAssets: { id: string; kind: string }[],
  knownEntityIds: string[],
  knownAnimationNames: string[],
): Promise<ChangeProposal> {
  return invoke("ai_orchestrate", {
    context,
    instruction,
    knownAssets,
    knownEntityIds,
    knownAnimationNames,
  });
}

/** Asks the AI Core to generate a whole game, or a whole new scene within
 * one, from a high-level instruction (milestone M21). A "Producer" decides
 * which scenes are needed (the one already open and/or brand-new ones);
 * each is built by the same pipeline as `orchestrateChange`. Returns one
 * validated `ChangeProposal` per scene, meant to be applied together. */
export function generateProject(
  context: string,
  instruction: string,
  knownAssets: { id: string; kind: string }[],
  knownSceneNames: string[],
  currentEntityIds: string[],
  currentAnimationNames: string[],
): Promise<ProjectProposal> {
  return invoke("ai_generate_project", {
    context,
    instruction,
    knownAssets,
    knownSceneNames,
    currentEntityIds,
    currentAnimationNames,
  });
}

/** Persists a script written by an accepted proposal into the project's
 * `assets/` directory and returns the resulting asset entry. */
export function writeScriptAsset(
  projectRoot: string,
  assetId: string,
  filename: string,
  content: string,
): Promise<{ id: string; path: string }> {
  return invoke("write_script_asset", {
    projectRoot,
    assetId,
    filename,
    content,
  });
}

const MIME_BY_EXTENSION: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  wav: "audio/wav",
  mp3: "audio/mpeg",
  ogg: "audio/ogg",
  flac: "audio/flac",
};

export const AUDIO_EXTENSIONS = ["wav", "mp3", "ogg", "flac"];

export async function readImageDataUrl(path: string): Promise<string> {
  const extension = path.split(".").pop()?.toLowerCase() ?? "png";
  const mime = MIME_BY_EXTENSION[extension] ?? "image/png";
  const base64 = await readFileBase64(path);
  return `data:${mime};base64,${base64}`;
}
