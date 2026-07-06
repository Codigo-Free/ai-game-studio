# Exportadores

Los exportadores convierten un proyecto `.aigs` en un producto distribuible por plataforma. Empaquetan el **runtime** junto con los datos y assets del proyecto.

---

## Plataformas

| Plataforma | Fase | Salida | Estado |
|---|---|---|---|
| **Desktop** (Linux, Windows, macOS) | 2 | Carpeta auto-contenida + `.zip` opcional. | 🟢 Disponible (M7) |
| **Web** | 3 | WASM + WebGPU/WebGL (el runtime ya usa WGPU, ver [runtime.md](runtime.md)). | 🟢 Disponible (M14) |
| **Android** | 3 | APK/AAB. | ⚪ |
| **iOS** | 3 | Proyecto Xcode / IPA. | ⚪ |

---

## Exportador Desktop (M7)

```bash
aigs export mi-juego/game.aigs --output dist --zip
```

Produce una carpeta lista para distribuir:

```
dist/mi-juego/
├── mi-juego[.exe]     # ejecutable del juego
└── data/
    ├── game.aigs      # manifiesto
    ├── scenes/…       # escenas del manifiesto
    └── assets/…       # assets del manifiesto
```

**Diseño self-player** (decisión en [arquitectura.md](arquitectura.md)): el binario `aigs` detecta al arrancar si existe `data/game.aigs` junto al ejecutable; si es así, ejecuta el juego directamente. Exportar consiste en copiar el ejecutable renombrado más los datos validados — sin compilación, sin toolchain para el usuario final.

- El editor expone el botón **⬇ Exportar** (guarda, elige carpeta y ejecuta `aigs export`).
- El exportador vive en `exporters/desktop` (`aigs-export-desktop`) y valida el proyecto completo antes de escribir nada; se niega a sobrescribir una exportación existente.
- Limitación M7: exporta para el **sistema operativo actual** (el binario empaquetado es el local). La exportación cruzada llega con los targets de Fase 3.

## Exportador Web (M14)

```bash
aigs export mi-juego/game.aigs --target web --output dist
```

Produce una carpeta servible como sitio estático:

```
dist/mi-juego/
├── index.html
├── aigs_web_player.js       # glue JS generado por wasm-bindgen
├── aigs_web_player_bg.wasm  # jugador genérico compilado a WASM
└── data/
    ├── game.aigs
    ├── scenes/…
    └── assets/…
```

Mismo diseño **self-player** que Desktop, adaptado a que en WASM no existe el equivalente de "el binario en ejecución": el jugador (`exporters/web-player`, crate `aigs-web-player`) es un único artefacto genérico que en vez de leer `data/game.aigs` del disco al arrancar, lo obtiene con `fetch` (URL relativa a donde se sirva `index.html`) — el resto de la carga de datos (`Project::from_json`, `Scene::from_json`, bytes de assets) es exactamente el mismo código que usa `aigs run`, solo cambia de dónde vienen los bytes (ver `aigs_runtime::AssetSource` en [arquitectura.md](arquitectura.md)).

- `aigs export --target web` **no compila nada**: copia un jugador ya compilado (`aigs_web_player.js`/`_bg.wasm`), esperado en `web-player/` junto al propio binario `aigs` — igual de "cero toolchain para quien exporta" que Desktop. Ese jugador se compila una sola vez con `wasm-bindgen` (ver CI, job *Web player*) y se distribuye junto al CLI.
- El renderer WGPU corre sobre WebGPU o WebGL2 según lo que el navegador soporte, sin cambios en el código del motor.
- Servir el resultado con cualquier servidor de archivos estáticos (`npx serve dist/mi-juego`); no funciona abriendo `index.html` con `file://` (los navegadores bloquean `fetch` sobre ese esquema).
- Limitaciones conocidas: sin `save.json` todavía (ver SPEC.md, sección de persistencia) y sin exportación desde el editor todavía (solo CLI; el botón del editor llega cuando el flujo esté validado en un navegador real).

## Diseño general

- Cada exportador vive en `exporters/` como módulo independiente; el pipeline común es: validar proyecto → obtener runtime del target → empaquetar datos → generar artefacto.
- En la Fase 5, la comunidad podrá aportar exportadores nuevos vía SDK ([plugins.md](plugins.md)).
