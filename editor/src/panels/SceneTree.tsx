import { useState } from "react";
import {
  generateId,
  insertEntity,
  removeEntity,
  reorderEntity,
  updateEntity,
} from "../document";
import { useStore } from "../store";
import type { EntityNode } from "../types";

function NodeRow({
  node,
  depth,
  onAction,
}: {
  node: EntityNode;
  depth: number;
  onAction: (action: string, node: EntityNode, value?: string) => void;
}) {
  const { state } = useStore();
  const [editing, setEditing] = useState(false);
  const selected = state.selection === node.id;
  const icon = node.components?.camera2d
    ? "🎥"
    : node.components?.sprite
      ? "🖼"
      : "▫";

  return (
    <>
      <div
        className={`tree-row${selected ? " selected" : ""}`}
        style={{ paddingLeft: 8 + depth * 16 }}
        onClick={() => onAction("select", node)}
        onDoubleClick={() => setEditing(true)}
      >
        <span className="tree-icon">{icon}</span>
        {editing ? (
          <input
            autoFocus
            defaultValue={node.name}
            onClick={(event) => event.stopPropagation()}
            onBlur={(event) => {
              setEditing(false);
              onAction("rename", node, event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") event.currentTarget.blur();
              if (event.key === "Escape") setEditing(false);
            }}
          />
        ) : (
          <span className="tree-name">{node.name}</span>
        )}
        {selected && !editing && (
          <span className="tree-actions">
            <button title="Subir" onClick={(e) => { e.stopPropagation(); onAction("up", node); }}>▲</button>
            <button title="Bajar" onClick={(e) => { e.stopPropagation(); onAction("down", node); }}>▼</button>
            <button title="Añadir hijo" onClick={(e) => { e.stopPropagation(); onAction("add-child", node); }}>＋</button>
            <button title="Eliminar" onClick={(e) => { e.stopPropagation(); onAction("delete", node); }}>✕</button>
          </span>
        )}
      </div>
      {(node.children ?? []).map((child) => (
        <NodeRow key={child.id} node={child} depth={depth + 1} onAction={onAction} />
      ))}
    </>
  );
}

export function SceneTree() {
  const { state, dispatch, currentScene } = useStore();

  if (!currentScene) {
    return <div className="panel-empty">Sin escena</div>;
  }

  const commitEntities = (entities: EntityNode[]) =>
    dispatch({
      type: "UPDATE_SCENE",
      scene: { ...currentScene, entities },
      commit: true,
    });

  const addEntity = (parentId: string | null) => {
    const id = generateId(currentScene, "entity");
    const node: EntityNode = {
      id,
      name: "Entidad",
      components: { transform2d: {} },
    };
    commitEntities(insertEntity(currentScene.entities, parentId, node));
    dispatch({ type: "SELECT", id });
  };

  const onAction = (action: string, node: EntityNode, value?: string) => {
    switch (action) {
      case "select":
        dispatch({ type: "SELECT", id: node.id });
        break;
      case "rename":
        if (value && value !== node.name) {
          commitEntities(
            updateEntity(currentScene.entities, node.id, (n) => ({
              ...n,
              name: value,
            })),
          );
        }
        break;
      case "delete":
        commitEntities(removeEntity(currentScene.entities, node.id));
        if (state.selection === node.id) dispatch({ type: "SELECT", id: null });
        dispatch({ type: "LOG", level: "info", message: `Entidad "${node.name}" eliminada` });
        break;
      case "up":
        commitEntities(reorderEntity(currentScene.entities, node.id, -1));
        break;
      case "down":
        commitEntities(reorderEntity(currentScene.entities, node.id, 1));
        break;
      case "add-child":
        addEntity(node.id);
        break;
    }
  };

  return (
    <div className="panel scene-tree">
      <div className="panel-header">
        Escena
        <button className="panel-header-action" onClick={() => addEntity(null)}>
          ＋ Entidad
        </button>
      </div>
      <div className="panel-body">
        {currentScene.entities.map((node) => (
          <NodeRow key={node.id} node={node} depth={0} onAction={onAction} />
        ))}
      </div>
    </div>
  );
}
