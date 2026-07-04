import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AUDIO_EXTENSIONS, importAsset, readImageDataUrl } from "../ipc";
import { useStore } from "../store";
import type { Asset } from "../types";

/** Shared cache of data-URLs for image assets (thumbnails + viewport). */
const imageCache = new Map<string, string>();

export function cachedImageUrl(key: string): string | undefined {
  return imageCache.get(key);
}

export async function ensureImageUrl(
  root: string,
  asset: Asset,
): Promise<string> {
  const key = `${root}/${asset.path}`;
  const hit = imageCache.get(key);
  if (hit) return hit;
  const url = await readImageDataUrl(key);
  imageCache.set(key, url);
  return url;
}

function AudioCard({ root, asset }: { root: string; asset: Asset }) {
  const [playing, setPlaying] = useState(false);
  const play = async () => {
    try {
      const url = await ensureImageUrl(root, asset);
      const audio = new Audio(url);
      setPlaying(true);
      audio.onended = () => setPlaying(false);
      void audio.play();
    } catch {
      setPlaying(false);
    }
  };
  return (
    <button className="thumb-placeholder audio-thumb" onClick={play} title="Pre-escuchar">
      {playing ? "🔊" : "▶🎵"}
    </button>
  );
}

function Thumbnail({ root, asset }: { root: string; asset: Asset }) {
  const [url, setUrl] = useState<string | undefined>(
    cachedImageUrl(`${root}/${asset.path}`),
  );
  useEffect(() => {
    let alive = true;
    if (!url && asset.kind === "image") {
      ensureImageUrl(root, asset)
        .then((loaded) => alive && setUrl(loaded))
        .catch(() => {});
    }
    return () => {
      alive = false;
    };
  }, [root, asset, url]);

  if (asset.kind === "audio") return <AudioCard root={root} asset={asset} />;
  return url ? (
    <img src={url} alt={asset.id} draggable={false} />
  ) : (
    <span className="thumb-placeholder">{asset.kind}</span>
  );
}

export function AssetsPanel() {
  const { state, dispatch } = useStore();
  const loaded = state.loaded;

  if (!loaded) return null;

  const doImport = async () => {
    try {
      const selected = await open({
        title: "Importar recursos",
        multiple: true,
        filters: [
          { name: "Imágenes y audio", extensions: ["png", "jpg", "jpeg", "gif", "webp", ...AUDIO_EXTENSIONS] },
        ],
      });
      const files =
        typeof selected === "string" ? [selected] : (selected ?? []);
      if (files.length === 0) return;
      const assets = [...loaded.project.assets];
      for (const file of files) {
        const imported = await importAsset(loaded.root, file);
        if (assets.some((a) => a.id === imported.id)) {
          dispatch({
            type: "LOG",
            level: "warn",
            message: `Asset "${imported.id}" ya existe, archivo sobrescrito`,
          });
          continue;
        }
        const extension = imported.path.split(".").pop()?.toLowerCase() ?? "";
        const kind = AUDIO_EXTENSIONS.includes(extension) ? "audio" : "image";
        assets.push({ id: imported.id, kind, path: imported.path });
        dispatch({
          type: "LOG",
          level: "info",
          message: `Asset "${imported.id}" importado (${imported.path})`,
        });
      }
      dispatch({ type: "UPDATE_ASSETS", assets });
    } catch (error) {
      dispatch({ type: "LOG", level: "error", message: String(error) });
    }
  };

  return (
    <div className="panel assets">
      <div className="panel-header">
        Recursos
        <button className="panel-header-action" onClick={doImport}>
          ＋ Importar
        </button>
      </div>
      <div className="panel-body asset-grid">
        {loaded.project.assets.length === 0 && (
          <div className="panel-empty">
            Importa imágenes y arrástralas a la escena
          </div>
        )}
        {loaded.project.assets.map((asset) => (
          <div
            key={asset.id}
            className="asset-card"
            draggable
            onDragStart={(event) => {
              event.dataTransfer.setData("aigs/asset-id", asset.id);
              event.dataTransfer.effectAllowed = "copy";
            }}
            title={`${asset.path} — arrastra al lienzo`}
          >
            <Thumbnail root={loaded.root} asset={asset} />
            <span className="asset-name">{asset.id}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
