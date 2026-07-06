# Arquitectura

AI Game Studio se compone de seis grandes bloques: **Editor, Runtime, Exporters, SDK, Plugins y AI Core**. Todos se comunican a través de un contrato común: el **formato de proyecto `.aigs`**.

---

## Principio rector

> Todo lo que el editor puede hacer se expresa como datos.

El proyecto completo (escenas, entidades, componentes, animaciones, assets, eventos) vive en archivos JSON legibles y versionados. Editor, runtime, exportadores e IA leen y escriben el mismo formato. Esto hace al sistema **AI-Ready**: un modelo de IA colabora manipulando datos, no simulando clics.

---

## Vista general

```
┌────────────────────────── Editor (Tauri) ──────────────────────────┐
│  React + TypeScript                                                │
│  Escena · Árbol · Inspector · Recursos · Timeline · Consola · IA   │
└───────────────▲────────────────────────────────▲───────────────────┘
                │ IPC (comandos/eventos)         │
┌───────────────▼───────────────┐   ┌────────────▼────────────┐
│        Runtime (Rust)         │   │   Proyecto .aigs (JSON) │
│  aigs-ecs · aigs-render(WGPU) │◄──│  escenas · entidades ·  │
│  aigs-anim · aigs-project     │   │  assets · animaciones   │
└───────────────▲───────────────┘   └────────────▲────────────┘
                │                                │
     ┌──────────┴──────────┐          ┌──────────┴──────────┐
     │      Exporters      │          │       AI Core       │
     │ Desktop·Android·Web │          │  Contexto + Agentes │
     └─────────────────────┘          └─────────────────────┘
```

---

## Módulos

### Editor
Aplicación de escritorio Tauri (Linux/Windows/macOS) con frontend React + TypeScript. Es la superficie visual del sistema: no contiene lógica de juego, opera sobre el modelo de documento y delega la ejecución al runtime embebido. Ver [editor.md](editor.md).

### Runtime
Motor de ejecución en Rust, organizado como workspace de crates:

| Crate | Responsabilidad |
|---|---|
| `aigs-ecs` | Entity Component System: entidades, componentes, sistemas, consultas. |
| `aigs-render` | Render 2D sobre WGPU: sprite batching, texturas, cámaras, capas. |
| `aigs-anim` | Evaluación de animaciones: pistas, keyframes, tweens, easing. |
| `aigs-project` | Carga/guardado y validación del formato `.aigs`; versionado y migraciones. |
| `aigs-runtime` | Capa de integración: componentes base, game loop, entrada, runner de ventana. |

Ver [runtime.md](runtime.md).

### Exporters
Convierten un proyecto `.aigs` en un binario distribuible por plataforma (Desktop en Fase 2; Android, Web e iOS en Fase 3). Ver [exportadores.md](exportadores.md).

### SDK
El contrato público del sistema: especificación del formato `.aigs`, APIs para plugins y bibliotecas cliente. Vive en `sdk/`.

### Plugins
Extensiones de comunidad sobre el SDK (Fase 5). Ver [plugins.md](plugins.md).

### AI Core
Capa que da a los modelos de IA (Ollama local; Claude, GPT y Gemini en cloud) acceso al contexto completo del proyecto y a acciones sobre el formato `.aigs`. Los agentes especializados se construyen sobre esta capa (Fase 4). Ver [ia.md](ia.md).

---

## Formato de proyecto `.aigs`

- **JSON** legible y autodescriptivo, con esquema y versión de formato explícita.
- Rutas de assets relativas a la raíz del proyecto.
- Un archivo de proyecto + un archivo por escena + catálogo de assets.
- Migraciones automáticas entre versiones del formato desde la Fase 1.
- SQLite se reserva para índices/caches locales del editor (miniaturas, búsqueda), nunca como fuente de verdad.

---

## Comunicación Editor ↔ Runtime

- El runtime corre embebido en el proceso Tauri.
- El frontend envía **comandos** (crear entidad, mover, play/stop) y recibe **eventos** (selección, logs, métricas) vía IPC de Tauri.
- El viewport de escena es renderizado por el runtime (WGPU); el prototipo técnico de esta integración es el primer paso del hito M3 del [plan](plan.md).

---

## Decisiones de arquitectura

Las decisiones relevantes se registran aquí antes de implementarse:

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-07 | Formato de proyecto JSON AI-Ready como contrato central | La IA y el editor deben operar sobre los mismos datos. |
| 2026-07 | ECS propio en lugar de bevy_ecs/hecs | Control total del diseño y del formato serializado; revisable si los benchmarks lo desaconsejan. |
| 2026-07 | WGPU como capa de render | Multiplataforma (Vulkan/Metal/DX12/WebGPU) y camino directo a Web en Fase 3. |
| 2026-07 | Tauri en lugar de Electron | Menor huella, backend Rust compartido con el runtime. |
| 2026-07 | Crate `aigs-runtime` como capa de integración (loop, componentes, runner) | Mantiene `aigs-ecs`/`aigs-render`/`aigs-anim` independientes y reutilizables. |
| 2026-07 | Game loop a paso fijo (60 Hz) con render interpolado vía `PrevTransform2D` | Simulación determinista independiente del refresco de pantalla. |
| 2026-07 | `wgpu` fijado en la serie 24 y `winit` en 0.30 durante el MVP | API estable conocida; la actualización a series nuevas se hará como tarea dedicada con benchmarks. |
| 2026-07 | En M1 el renderer dibuja a la ventana (winit); render offscreen para el viewport del editor se decide en M3 | Evita diseñar la integración editor↔runtime antes del prototipo de M3. |
| 2026-07 | **Viewport de edición en Canvas 2D (frontend), runtime real solo en Play** | El viewport de diseño dibuja el modelo de documento con la misma matemática TRS que el runtime; el botón Play guarda y lanza `aigs run` (WGPU real). Embeber WGPU bajo el webview de Tauri es frágil multiplataforma; se reevaluará para el modo Play integrado (M5). |
| 2026-07 | El modelo de documento vive en el frontend; el backend Tauri solo toca disco y procesos | Cada guardado se revalida con `aigs-project` (la implementación de referencia), garantizando que el editor nunca escribe un `.aigs` inválido. |
| 2026-07 | **Exportación desktop con diseño self-player**: el binario `aigs` ejecuta `data/game.aigs` si existe junto al ejecutable; exportar = copiar ejecutable + datos | Un solo binario (CLI + player + juego exportado), cero compilación para el usuario final, y el mismo artefacto ya probado en `aigs run`. Exportación cruzada se resolverá en Fase 3 con binarios por target. |
| 2026-07 | **Persistencia (M13): `save.json` fuera del formato `.aigs`, guarda solo la memoria de scripts (`get_var`/`set_var`) por id de entidad + marca de tiempo** | El estado de juego (partida) y los datos de diseño (proyecto) son cosas distintas; mezclarlos en `.aigs` rompería el principio de que el formato es contenido de diseño estático. Guardar solo la memoria de scripts (no todo el `World`) evita construir serialización genérica del ECS antes de que haya un caso de uso real que la necesite. |
| 2026-07 | **`elapsed_since_save()` se consume una sola vez por sesión (primera llamada de cualquier script; el resto reciben 0), no un valor que crece cada tick** | El tiempo offline se debe aplicar analíticamente una vez (p. ej. en `on_start`), no una y otra vez cada frame ni recalcular reproduciendo cada tick perdido — eso sería lento y frágil. Consumo único evita doble conteo aunque el script se lea desde varias entidades o tras cambiar de escena. |
| 2026-07 | La memoria de scripts (`get_var`/`set_var`) sobrevive a un cambio de escena dentro de la misma sesión, indexada por id de entidad autor | Antes se perdía al salir de la escena (bug de usabilidad para cualquier juego con más de una escena); ahora el guardado a disco y la persistencia entre escenas comparten la misma estructura interna en `ScriptHost`. |
| 2026-07 | Autoguardado periódico (sin hook de cierre limpio de ventana todavía) | Evita añadir una ruta de apagado especial en `aigs-runtime::run()` antes de que haya más de un caso de uso; pérdida máxima aceptada: la ventana entre autoguardados. |
| 2026-07 | `save.json` vive junto a los datos del proyecto, no en un directorio de perfil de usuario | Suficiente para el MVP de la demo; un directorio de guardado por usuario (XDG/AppData) queda para cuando haya más juegos que lo necesiten. |
