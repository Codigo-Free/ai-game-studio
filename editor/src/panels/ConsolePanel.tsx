import { useEffect, useRef } from "react";
import { useStore } from "../store";

export function ConsolePanel() {
  const { state, dispatch } = useStore();
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const body = bodyRef.current;
    if (body) body.scrollTop = body.scrollHeight;
  }, [state.logs]);

  return (
    <div className="panel console">
      <div className="panel-header">
        Consola
        <button
          className="panel-header-action"
          onClick={() => dispatch({ type: "CLEAR_LOGS" })}
        >
          Limpiar
        </button>
      </div>
      <div className="panel-body console-body" ref={bodyRef}>
        {state.logs.map((entry, index) => (
          <div key={index} className={`log log-${entry.level}`}>
            <span className="log-time">{entry.time}</span>
            <span className="log-message">{entry.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
