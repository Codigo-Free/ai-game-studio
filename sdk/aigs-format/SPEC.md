# Especificación del formato `.aigs` — versión 0

**Estado:** borrador activo (hito M0–M2 del [plan](../../docs/plan.md)).
**Implementación de referencia:** crate [`aigs-project`](../../runtime/crates/aigs-project/).

El formato `.aigs` es el **contrato AI-Ready** central de AI Game Studio: todo lo que el editor puede hacer se expresa como estos datos, y editor, runtime, exportadores y agentes de IA leen y escriben exactamente el mismo formato. Sus prioridades de diseño son, en orden: legibilidad (humana y por LLM), estabilidad con migraciones, y extensibilidad.

---

## Convenciones generales

- Codificación **JSON** (UTF-8), claves en `snake_case`.
- Rutas de archivo **relativas a la raíz del proyecto**, separador `/`.
- Todo documento comienza con una cabecera `format` que declara su tipo y versión:

```json
{ "format": { "kind": "aigs-project", "version": 0 } }
```

- Un lector debe **rechazar** versiones mayores a las que soporta y **migrar** versiones menores.
- Claves desconocidas deben **preservarse** al reescribir un documento (round-trip sin pérdida), lo que permite componentes de plugins con namespace propio (ej. `"mi_plugin.iman"`).

## Estructura de un proyecto en disco

```
mi-juego/
├── game.aigs                  # manifiesto del proyecto (kind: aigs-project)
├── scenes/
│   ├── main.scene.aigs        # una escena por archivo (kind: aigs-scene)
│   └── level1.scene.aigs
└── assets/
    └── hero.png
```

---

## Documento 1: manifiesto del proyecto (`game.aigs`)

```json
{
  "format": { "kind": "aigs-project", "version": 0 },
  "name": "Robot Rescue",
  "description": "Un robot debe rescatar a su mascota.",
  "initial_scene": "scenes/main.scene.aigs",
  "scenes": [
    "scenes/main.scene.aigs",
    "scenes/level1.scene.aigs"
  ],
  "assets": [
    { "id": "hero", "kind": "image", "path": "assets/hero.png" }
  ]
}
```

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `format` | header | sí | `kind` debe ser `"aigs-project"`. |
| `name` | string | sí | Nombre del juego. |
| `description` | string | no | Descripción libre (contexto valioso para la IA). |
| `initial_scene` | string | sí | Escena de arranque; **debe** estar listada en `scenes`. |
| `scenes` | string[] | sí | Rutas de los archivos de escena. |
| `assets` | Asset[] | no (`[]`) | Catálogo de recursos. |

### Asset

| Campo | Tipo | Descripción |
|---|---|---|
| `id` | string | Único en el proyecto; referenciado por componentes (`sprite.asset`). |
| `kind` | enum | `"image"` \| `"audio"` \| `"font"` \| `"other"`. |
| `path` | string | Ruta relativa al archivo. |

---

## Documento 2: escena (`*.scene.aigs`)

```json
{
  "format": { "kind": "aigs-scene", "version": 0 },
  "name": "main",
  "entities": [
    {
      "id": "hero",
      "name": "Hero",
      "components": {
        "transform2d": { "x": 100.0, "y": 200.0, "rotation": 0.0, "scale_x": 1.0, "scale_y": 1.0 },
        "sprite": { "asset": "hero", "opacity": 1.0, "layer": 1 }
      },
      "children": []
    }
  ],
  "animations": [
    {
      "name": "intro",
      "fps": 30,
      "loop": true,
      "tracks": [
        {
          "entity": "hero",
          "property": "transform2d.x",
          "keyframes": [
            { "frame": 0, "value": 0.0, "easing": "linear" },
            { "frame": 30, "value": 100.0, "easing": "ease_in_out" }
          ]
        }
      ]
    }
  ]
}
```

### Entidad (`EntityNode`)

| Campo | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `id` | string | sí | Único dentro de la escena; referenciado por pistas de animación. |
| `name` | string | sí | Nombre visible en el árbol del editor. |
| `components` | objeto | no (`{}`) | Mapa `nombre → componente` (ver abajo). |
| `children` | EntityNode[] | no (`[]`) | Hijos en el árbol de escena (transformaciones relativas al padre). |

### Componentes de la versión 0

**`transform2d`** — posición, rotación (grados, horario) y escala. Todos los campos con default (`0` / escala `1`).

**`sprite`** — `asset` (id de un asset `image`, obligatorio), `width`/`height` (tamaño base en unidades de mundo; default: tamaño de la textura), `opacity` (default `1.0`), `layer` (entero, mayor = encima, default `0`).

**`camera2d`** — `zoom` (default `1.0`).

**Componentes de plugin:** cualquier otra clave con namespace (`"mi_plugin.iman"`) es válida y debe preservarse aunque el lector no la entienda.

### Animaciones

| Campo | Tipo | Descripción |
|---|---|---|
| `name` | string | Único en la escena. |
| `fps` | entero | Fotogramas por segundo del timeline. |
| `loop` | bool | Default `false`. |
| `tracks` | Track[] | Pistas de animación. |

**Track:** `entity` (id de entidad), `property` (ruta de propiedad animable, ej. `"transform2d.x"`, `"sprite.opacity"`), `keyframes`.

**Keyframe:** `frame` (entero ≥ 0), `value` (número), `easing` hacia el siguiente keyframe: `"linear"` (default) | `"ease_in"` | `"ease_out"` | `"ease_in_out"`.

**Propiedades animables en v0:** `transform2d.x`, `transform2d.y`, `transform2d.rotation`, `transform2d.scale_x`, `transform2d.scale_y`, `sprite.opacity`.

### Semántica de reproducción

- **Todas las animaciones de la escena comienzan al cargarse la escena.** (Los disparadores por eventos llegan en M5.)
- Una animación con `loop: true` se repite indefinidamente (el tiempo hace *wrap* sobre su duración); con `loop: false` se reproduce una vez y **mantiene el valor final**.
- La duración de una animación es el frame más alto entre sus keyframes; antes del primer keyframe rige su valor, después del último rige el del último.
- Pistas que referencian entidades o propiedades desconocidas se ignoran con una advertencia (no son error fatal).
- Implementación de referencia: `aigs_anim::sample` + `aigs_runtime::AnimationPlayback`; el editor usa un espejo TypeScript (`editor/src/anim.ts`) que debe mantenerse en sync.

---

## Validación

La herramienta CLI valida un proyecto completo:

```bash
aigs validate mi-juego/game.aigs
```

Comprueba: JSON bien formado, cabeceras y versiones, `initial_scene` listada, escenas cargables y archivos de assets existentes.

## Evolución del formato

- Los cambios de esquema incrementan `version` y añaden una migración en `aigs-project`.
- Cambios pendientes conocidos: spritesheets/frames de sprite (M4), eventos y comportamientos (M5), física/audio/partículas (Fase 2).
