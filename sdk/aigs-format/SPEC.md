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
| `kind` | enum | `"image"` \| `"audio"` \| `"font"` \| `"script"` (rhai, M12) \| `"other"`. |
| `path` | string | Ruta relativa al archivo. |
| `spritesheet` | objeto | Opcional (M10): `{ "frame_width": N, "frame_height": N }` convierte la imagen en rejilla de frames (row-major desde arriba-izquierda; columnas/filas se derivan del tamaño de la textura). |

---

## Documento 2: escena (`*.scene.aigs`)

```json
{
  "format": { "kind": "aigs-scene", "version": 0 },
  "name": "main",
  "gravity": { "x": 0.0, "y": -980.0 },
  "music": { "asset": "theme", "volume": 0.8, "looped": true },
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

**`sprite`** — `asset` (id de un asset `image`, obligatorio), `frame` (índice de frame del spritesheet, default `0`), `width`/`height` (tamaño base en unidades de mundo; default: tamaño del frame si hay spritesheet, si no el de la textura), `opacity` (default `1.0`), `layer` (entero, mayor = encima, default `0`).

**`camera2d`** — `zoom` (default `1.0`).

**`rigidbody2d`** — cuerpo físico (M8): `body` = `"dynamic"` (simulado, default) | `"kinematic"` (dirigido por transform, empuja a los dinámicos) | `"static"` (inmóvil); `gravity_scale` (default `1`), `vx`/`vy` (velocidad inicial, unidades/s), `fixed_rotation` (default `false`). Requiere `collider2d`.

**`collider2d`** — forma de colisión (M8): `shape` = `"box"` (default) | `"circle"`; `width`/`height` o `radius` (default: tamaño visible del sprite); `sensor` (detecta sin bloquear, default `false`); `restitution` (rebote 0–1, default `0`); `friction` (default `0.5`). Sin `rigidbody2d` actúa como colisionador estático.

**`animator`** — máquina de estados de animación (M10): `initial` (estado de arranque), `states` (mapa `estado → nombre de animación de la escena`) y `transitions` (lista de `{ "from": estado | "any", "to": estado, "when": Evento }`; eventos soportados: teclas, `scene_start`, `animation_end`). **Las animaciones referenciadas por un animator no se auto-reproducen** — las controla la máquina, empezando por `initial`; al salir de un estado su animación se detiene y al entrar se reinicia.

**`particles`** — emisor de partículas (M11): `asset` (imagen, obligatorio), `rate` (partículas/s mientras `emitting`, default `20`; `0` = solo ráfagas), `lifetime` (s, default `0.8`), `speed` (unidades/s, default `120`), `direction` (grados, `90` = arriba), `spread` (arco en grados centrado en `direction`, default `360`), `gravity` (aceleración vertical, default `0`), `start_scale`/`end_scale` (default `1`/`0.2`), `start_opacity`/`end_opacity` (default `1`/`0`), `layer` (default `5`), `emitting` (default `true`). Las partículas se simulan en el runtime y **no** forman parte del documento.

**`script`** — script de usuario (M12): `{ "asset": id }` donde el asset es de tipo `script` (archivo `.rhai`). Ver sección Scripting.

**`behaviors`** — lista de reglas sin código `{ "on": Evento, "do": Acción }` (ver sección Comportamientos).

**`virtual_button`** — botón táctil en pantalla (M15): `{ "key": nombre }`. Mientras se mantiene tocado el sprite de la entidad, simula esa tecla como pulsada para behaviors, animators y scripts — el mismo evento que una tecla física, así que un proyecto pensado para teclado funciona en pantalla táctil sin más cambios que añadir este componente a un sprite. Se libera en cuanto el dedo se levanta o se mueve fuera del sprite.

**Componentes de plugin:** cualquier otra clave con namespace (`"mi_plugin.iman"`) es válida y debe preservarse aunque el lector no la entienda.

### Comportamientos (v0)

```json
"behaviors": [
  { "on": { "type": "key_down", "key": "ArrowRight" },
    "do": { "type": "move", "dx": 300.0, "dy": 0.0 } },
  { "on": { "type": "click" },
    "do": { "type": "goto_scene", "scene": "scenes/level1.scene.aigs" } }
]
```

**Eventos** (`on.type`):

| Evento | Parámetros | Dispara |
|---|---|---|
| `key_down` | `key` | Cada tick mientras la tecla está pulsada (continuo). |
| `key_pressed` | `key` | El tick en que la tecla baja. |
| `key_released` | `key` | El tick en que la tecla sube (M10). |
| `click` | — | Clic izquierdo sobre el sprite de la entidad (hit-test con rotación/escala, capa superior gana). |
| `scene_start` | — | Una vez, al cargar la escena. |
| `animation_end` | `animation` | Cuando una animación sin loop termina. |
| `collision` | `with` (opcional) | Cuando esta entidad empieza a tocar otro colisionador (M8); `with` filtra por id de entidad. |

Nombres de tecla: estilo winit/W3C — `ArrowLeft/Right/Up/Down`, `Space`, `Enter`, `Escape`, `Tab`, letras (`a`/`KeyA`) y dígitos (`1`/`Digit1`).

**Táctil (M15):** en Android (y cualquier pantalla táctil) un dedo se trata igual que el ratón — mover el dedo mueve el "cursor" y tocar dispara `click` exactamente igual que un clic izquierdo. Solo se sigue el primer dedo (sin multitáctil todavía). Para movimiento por teclado (`key_down`/`key_pressed`) sin teclado físico, usa `virtual_button`.

**Acciones** (`do.type`):

| Acción | Parámetros | Efecto |
|---|---|---|
| `move` | `dx`, `dy` | Con eventos continuos (`key_down`): unidades/segundo. Con eventos discretos: desplazamiento instantáneo. |
| `goto_scene` | `scene` | Cambia a otra escena del manifiesto (al final del tick; el mundo se repuebla). |
| `play_animation` | `animation` | Reinicia una animación de la escena por nombre. |
| `play_sound` | `asset`, `volume` (default `1.0`) | Reproduce un efecto de sonido (asset `audio`), M9. |
| `emit_particles` | `count` (default `20`) | Ráfaga desde el emisor de la propia entidad (M11); requiere componente `particles`. |

Reglas con teclas, entidades, escenas o animaciones desconocidas generan advertencia y se ignoran (no son error fatal).

### Animaciones

| Campo | Tipo | Descripción |
|---|---|---|
| `name` | string | Único en la escena. |
| `fps` | entero | Fotogramas por segundo del timeline. |
| `loop` | bool | Default `false`. |
| `tracks` | Track[] | Pistas de animación. |

**Track:** `entity` (id de entidad), `property` (ruta de propiedad animable, ej. `"transform2d.x"`, `"sprite.opacity"`), `keyframes`.

**Keyframe:** `frame` (entero ≥ 0), `value` (número), `easing` hacia el siguiente keyframe: `"linear"` (default) | `"ease_in"` | `"ease_out"` | `"ease_in_out"`.

**Propiedades animables en v0:** `transform2d.x`, `transform2d.y`, `transform2d.rotation`, `transform2d.scale_x`, `transform2d.scale_y`, `sprite.opacity`, `sprite.frame` (el valor se trunca a entero al aplicarse; anima `0 → N-0.1` para recorrer N frames).

### Semántica de reproducción

- **Todas las animaciones de la escena comienzan al cargarse la escena.** (Los disparadores por eventos llegan en M5.)
- Una animación con `loop: true` se repite indefinidamente (el tiempo hace *wrap* sobre su duración); con `loop: false` se reproduce una vez y **mantiene el valor final**.
- La duración de una animación es el frame más alto entre sus keyframes; antes del primer keyframe rige su valor, después del último rige el del último.
- Pistas que referencian entidades o propiedades desconocidas se ignoran con una advertencia (no son error fatal).
- Implementación de referencia: `aigs_anim::sample` + `aigs_runtime::AnimationPlayback`; el editor usa un espejo TypeScript (`editor/src/anim.ts`) que debe mantenerse en sync.

---

### Scripting (v0, M12–M13)

Los scripts son archivos **rhai** sandboxeados (sin IO, con presupuesto de operaciones por tick — un script fuera de control se degrada a advertencia, nunca cuelga el juego). Un script puede definir cuatro funciones de ciclo de vida, todas opcionales:

| Función | Cuándo se llama |
|---|---|
| `fn on_start()` | Una vez, justo al cargar la escena. |
| `fn on_update(dt)` | Cada tick de simulación (dt en segundos). |
| `fn on_collision(other)` | Cuando el colisionador de esta entidad empieza a tocar otro; `other` es el id de la entidad tocada (`""` si no tiene). |
| `fn on_destroy()` | Una vez, justo antes de que la escena se destruya (p. ej. al cambiar de escena). |

**Contrato tipado y máquina-legible:** la API completa (cada función, sus parámetros tipados y su descripción) vive en `aigs_runtime::api_manifest()` y se publica como snapshot en [`scripting-api.json`](scripting-api.json) (regenerar con `aigs script-api > sdk/aigs-format/scripting-api.json`); un test de integración falla si el snapshot queda desactualizado respecto al motor.

**Estado persistente — `get_var`/`set_var`, no `this`:** las funciones (`fn`) de rhai no ven variables externas entre llamadas, así que un script que quiera recordar algo entre ticks debe usar `get_var(nombre)` / `set_var(nombre, valor)` (memoria float por instancia, no por variable capturada). Se reinicia si el script se recarga en caliente o la escena reinicia.

| Función | Categoría | Descripción |
|---|---|---|
| `x()`, `y()`, `rotation()`, `frame()` | Estado propio | Transform/frame de la propia entidad. |
| `get_var(nombre)`, `set_var(nombre, valor)` | Estado persistente | Memoria de esta instancia entre ticks. |
| `elapsed_since_save()` | Estado persistente | Segundos reales transcurridos desde el último autoguardado (0.0 si no hay guardado previo o ya se consumió). **Se consume al llamarla**: solo el primer script que la invoca en la sesión recibe el valor real; todas las llamadas siguientes (misma entidad u otra, misma escena u otra) devuelven `0.0`. Pensado para leerse una vez en `on_start`. |
| `x_of(id)`, `y_of(id)`, `distance_to(id)` | Otras entidades | Posiciones por id de entidad autor. |
| `key_down(n)`, `key_pressed(n)`, `key_released(n)` | Entrada | Mismos nombres de tecla que los behaviors. |
| `set_pos(x, y)`, `move_by(dx, dy)`, `set_rotation(r)`, `set_frame(f)`, `set_scale(sx, sy)` | Transform | Mutan la propia entidad (se aplican al terminar la llamada). |
| `goto_scene(ruta)`, `play_animation(nombre)`, `play_sound(id[, vol])`, `emit_particles(n)` | Motor | `goto_scene` se ignora (con advertencia) si se llama desde `on_destroy`. |
| `log(msg)` | Utilidad | Escribe en la consola (`[script:asset] msg`). |

Errores de compilación o ejecución se reportan como advertencias (visibles en la consola del editor); el script fallido se desactiva sin tumbar el juego.

**Hot reload:** mientras el juego corre desde un directorio de proyecto (no exportado), el runtime revisa el `mtime` de cada `.rhai` un par de veces por segundo; si cambió, lo recompila y reinicia el estado local de sus instancias (`get_var`/`set_var` y si ya se ejecutó `on_start`). Editar y guardar un script mientras el modo Play está corriendo lo actualiza sin reiniciar la partida. Un error de compilación durante la recarga conserva la versión anterior (que sigue funcionando) y lo reporta como advertencia.

**Memoria entre escenas:** la memoria `get_var`/`set_var` de cada instancia de script se identifica por el **id de entidad autor** (no por el handle de ECS, que no es estable entre escenas) y sobrevive a un `goto_scene`: si la escena destino tiene una entidad con el mismo id y el mismo script, retoma su memoria donde la dejó.

### Persistencia entre partidas — `save.json` (M13)

El motor puede recordar el estado de un juego **entre ejecuciones reales** (cerrar y volver a abrir), no solo entre escenas. Esto vive fuera del formato `.aigs` deliberadamente: `save.json` es estado de partida de un jugador concreto, no dato de diseño, así que no se versiona con el proyecto ni lo toca el editor.

- `aigs run` escribe `save.json` junto a `game.aigs` cada ~10 s de simulación (autoguardado periódico; no hay todavía un gancho de cierre limpio, así que en el peor caso se pierden hasta ~10 s de progreso).
- Al arrancar, si existe `save.json`, se restaura la memoria de todos los scripts (`get_var`/`set_var` por id de entidad) y se calcula el tiempo real transcurrido desde `saved_at_unix`, disponible una única vez vía `elapsed_since_save()`.
- Si `save.json` no existe, la partida arranca limpia (no es un error). Si existe pero está corrupto, se reporta por consola y también arranca limpia (nunca se ignora en silencio un archivo dañado).
- Ubicación deliberada: junto a los datos del proyecto (no en un directorio de perfil de usuario), coherente con el diseño self-player de los exportados (M7).
- Estructura: `{"version": 1, "saved_at_unix": <u64>, "scripts": {"<id de entidad>": {"<variable>": <f64>, ...}, ...}}`.
- **Limitación conocida (M14/M15):** los exportadores Web y Android no tienen todavía un `save.json` equivalente (Web no tiene sistema de archivos; Android no lee/escribe fuera de sus propios assets empaquetados en el APK); `elapsed_since_save()` siempre devuelve `0` en ambos. Un juego que dependa de persistencia real de partida (como el Tamagotchi de M13) no la tiene aún al exportarse a Web o Android.

### Audio (v0, M9)

- `music` a nivel de escena: `asset` (id de un asset `audio`), `volume` lineal 0–1 (default `1.0`), `looped` (default `true`).
- La música arranca al cargar la escena; **si la escena siguiente declara el mismo asset, la música continúa sin cortarse**; si declara otro (o ninguno), se detiene/cambia.
- Formatos soportados por el runtime: WAV (mp3/ogg/flac ampliables por features de kira).
- Sin dispositivo de audio (CI, headless) el juego corre igual con el audio deshabilitado (warning, no error).

### Física (v0, M8)

- `gravity` a nivel de escena (default `{x: 0, y: -980}` unidades/s²) afecta solo a cuerpos dinámicos.
- La simulación corre al paso fijo del runtime (60 Hz) con rapier2d.
- Cuerpos **dinámicos**: la física escribe su `transform2d` cada tick (posición y rotación, salvo `fixed_rotation`).
- Cuerpos **kinemáticos**: el transform (behaviors, animaciones) manda; empujan a los dinámicos.
- Limitación v0: la acción `move` sobre un cuerpo **dinámico** no altera la simulación (la física lo sobreescribe); usa cuerpos kinemáticos para movimiento controlado.

## Validación

La herramienta CLI valida un proyecto completo:

```bash
aigs validate mi-juego/game.aigs
```

Comprueba: JSON bien formado, cabeceras y versiones, `initial_scene` listada, escenas cargables y archivos de assets existentes.

## Evolución del formato

- Los cambios de esquema incrementan `version` y añaden una migración en `aigs-project`.
- Cambios pendientes conocidos: curvas de easing personalizadas (bezier), diferidas sin fecha (M13 pasó a ser persistencia + demo Tamagotchi).
