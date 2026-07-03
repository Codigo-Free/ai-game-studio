// Bridge to the Tauri backend (editor/src-tauri/src/lib.rs).

import { invoke } from "@tauri-apps/api/core";
import type { LoadedProject, Project, Scene } from "./types";

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

const MIME_BY_EXTENSION: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
};

export async function readImageDataUrl(path: string): Promise<string> {
  const extension = path.split(".").pop()?.toLowerCase() ?? "png";
  const mime = MIME_BY_EXTENSION[extension] ?? "image/png";
  const base64 = await readFileBase64(path);
  return `data:${mime};base64,${base64}`;
}
