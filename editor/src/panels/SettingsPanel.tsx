import { useEffect, useState } from "react";
import { getAiSettings, saveAiSettings } from "../ipc";
import type { AiProvider, AiSettings } from "../types";

interface SettingsPanelProps {
  onClose: () => void;
}

/** Modal for the AI provider settings (fast-follow of M18): lets the user
 * pick Ollama/Claude/OpenAI and its model/API key without touching
 * environment variables. Saved to a local per-user file — see
 * `editor/src-tauri/src/settings.rs`. */
export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getAiSettings().then(setSettings);
  }, []);

  const update = (patch: Partial<AiSettings>) => {
    setSettings((current) => (current ? { ...current, ...patch } : current));
  };

  const save = async () => {
    if (!settings) return;
    setSaving(true);
    setError(null);
    try {
      await saveAiSettings(settings);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(event) => event.stopPropagation()}>
        <div className="settings-header">
          <h3>Ajustes de IA</h3>
          <button className="panel-header-action" onClick={onClose}>
            ✕
          </button>
        </div>
        {!settings ? (
          <div className="settings-body">Cargando…</div>
        ) : (
          <>
            <div className="settings-body">
              <div className="field">
                <span>Proveedor</span>
                <select
                  value={settings.provider}
                  onChange={(event) =>
                    update({ provider: event.target.value as AiProvider })
                  }
                >
                  <option value="ollama">Ollama (local)</option>
                  <option value="claude">Claude (cloud)</option>
                  <option value="openai">OpenAI (cloud)</option>
                </select>
              </div>

              {settings.provider === "ollama" && (
                <>
                  <div className="field">
                    <span>URL base</span>
                    <input
                      type="text"
                      value={settings.ollama_base_url}
                      onChange={(event) =>
                        update({ ollama_base_url: event.target.value })
                      }
                    />
                  </div>
                  <div className="field">
                    <span>Modelo</span>
                    <input
                      type="text"
                      value={settings.ollama_model}
                      onChange={(event) =>
                        update({ ollama_model: event.target.value })
                      }
                    />
                  </div>
                </>
              )}

              {settings.provider === "claude" && (
                <>
                  <div className="field">
                    <span>API Key</span>
                    <input
                      type="password"
                      value={settings.claude_api_key}
                      onChange={(event) =>
                        update({ claude_api_key: event.target.value })
                      }
                      placeholder="sk-ant-…"
                    />
                  </div>
                  <div className="field">
                    <span>Modelo</span>
                    <input
                      type="text"
                      value={settings.claude_model}
                      onChange={(event) =>
                        update({ claude_model: event.target.value })
                      }
                    />
                  </div>
                </>
              )}

              {settings.provider === "openai" && (
                <>
                  <div className="field">
                    <span>API Key</span>
                    <input
                      type="password"
                      value={settings.openai_api_key}
                      onChange={(event) =>
                        update({ openai_api_key: event.target.value })
                      }
                      placeholder="sk-…"
                    />
                  </div>
                  <div className="field">
                    <span>Modelo</span>
                    <input
                      type="text"
                      value={settings.openai_model}
                      onChange={(event) =>
                        update({ openai_model: event.target.value })
                      }
                    />
                  </div>
                </>
              )}

              <p className="hint">
                La API key se guarda en texto plano en un archivo local de tu
                usuario, no en un llavero del sistema. Si tenés una variable
                de entorno seteada (AIGS_AI_PROVIDER, ANTHROPIC_API_KEY,
                OPENAI_API_KEY…), esa tiene prioridad sobre esto.
              </p>
              {error && <p className="settings-error">{error}</p>}
            </div>
            <div className="settings-footer">
              <button onClick={onClose} disabled={saving}>
                Cancelar
              </button>
              <button onClick={() => void save()} disabled={saving}>
                {saving ? "Guardando…" : "Guardar"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
