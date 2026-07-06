import { useEffect, useRef, useState } from "react";
import { askAi } from "../ipc";
import { useStore } from "../store";

// Cap on how much of the project we hand to the model — a crude but
// simple guard against blowing past a smaller local model's context
// window on a large project (milestone M18; no smarter summarization
// yet, see docs/plan.md).
const MAX_CONTEXT_CHARS = 12000;

interface ChatEntry {
  role: "user" | "assistant" | "error";
  text: string;
}

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
  const { state, currentScene } = useStore();
  const [history, setHistory] = useState<ChatEntry[]>([]);
  const [question, setQuestion] = useState("");
  const [asking, setAsking] = useState(false);
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const body = bodyRef.current;
    if (body) body.scrollTop = body.scrollHeight;
  }, [history]);

  const ask = async () => {
    const trimmed = question.trim();
    if (!trimmed || asking) return;
    setHistory((h) => [...h, { role: "user", text: trimmed }]);
    setQuestion("");
    setAsking(true);
    try {
      const context = buildContext(
        state.loaded?.project ?? null,
        state.currentScenePath,
        currentScene,
      );
      const answer = await askAi(context, trimmed);
      setHistory((h) => [...h, { role: "assistant", text: answer }]);
    } catch (error) {
      setHistory((h) => [...h, { role: "error", text: String(error) }]);
    } finally {
      setAsking(false);
    }
  };

  return (
    <div className="panel chat">
      <div className="panel-header">
        Chat
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
            Pregunta algo sobre el proyecto abierto, por ejemplo "¿qué
            comportamientos tiene la entidad robot?".
          </p>
        )}
        {history.map((entry, index) => (
          <div key={index} className={`chat-entry chat-${entry.role}`}>
            {entry.text}
          </div>
        ))}
        {asking && <div className="chat-entry chat-assistant chat-pending">…</div>}
      </div>
      <form
        className="chat-input"
        onSubmit={(event) => {
          event.preventDefault();
          void ask();
        }}
      >
        <input
          type="text"
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          placeholder="Pregunta sobre el proyecto…"
          disabled={!state.loaded || asking}
        />
        <button type="submit" disabled={!state.loaded || asking}>
          Enviar
        </button>
      </form>
    </div>
  );
}
