# CLAUDE.md — AI Game Studio

Guía de contexto para asistentes de IA que trabajen en este repositorio. Este proyecto se desarrolla con metodología **AI First**: la IA participa en diseño, código, documentación, pruebas y revisión.

## Qué es este proyecto

Plataforma **AI-First open source** para crear videojuegos 2D combinando el paradigma visual de Adobe Flash (timeline, fotogramas) con tecnología moderna (Rust, ECS, WGPU, Tauri) e IA nativa. Lema: *Build Games at the Speed of Imagination*. Principio central: **la IA conoce el videojuego y el videojuego conoce la IA** — todo el proyecto de juego vive en el formato `.aigs` (JSON AI-Ready) que editor, runtime, exportadores e IA comparten como contrato.

## Estado actual

- **Fase:** 1 (MVP) — **M0, M1 y M2 completados; M3 con código completo** (pendiente validación manual del editor); siguiente: validar M3 y arrancar **M4 (Timeline)**.
- M3 entregó: editor funcional — backend Tauri (`load/create/save_project` revalidando con `aigs-project`, `import_asset`, `play_project` que lanza `aigs run`), modelo de documento en React con undo/redo por snapshots, viewport Canvas 2D (selección, arrastre, zoom, pan, drop de assets, marcadores de cámara), árbol de escena (crear/renombrar/eliminar/reordenar/hijos), inspector (transform/sprite/camera), panel de recursos con miniaturas e importación, consola y atajos (Ctrl+S/Z/Shift+Z, Supr).
- **Para ejecutar el editor:** `cd editor && npm run tauri dev`. Requiere `webkit2gtk-4.1` del sistema (`sudo pacman -S webkit2gtk-4.1`) — no está instalado en esta máquina, por eso la validación manual está pendiente. El botón Play necesita el CLI: `cargo install --path cli` o `AIGS_CLI=<ruta>`.
- Decisión clave M3: viewport de edición en Canvas 2D (misma matemática TRS que el runtime); el runtime WGPU real corre en Play vía `aigs run`. Ver tabla en `docs/arquitectura.md`.
- Deuda técnica: `wgpu` 24 / `winit` 0.30 (actualizar tras MVP); jerarquía aplanada al instanciar en runtime; docking fijo (CSS grid) sin paneles arrastrables; animaciones no se reproducen aún (M4).
- Gotcha: en Wayland+EGL el `Renderer` debe soltarse en `exiting()` de winit o segfaultea (resuelto en `aigs-runtime/src/app.rs`).

## Mapa del repositorio

| Ruta | Contenido |
|---|---|
| `docs/` | Documentación de diseño (fuente de verdad del proyecto). |
| `editor/` | Editor visual — Tauri 2 + React + Vite + TypeScript (`npm run build` / `npm run tauri dev`). |
| `runtime/crates/` | Crates del motor: `aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`. |
| `cli/` | Binario `aigs`: `validate <game.aigs>` y `run <game.aigs>` (ejecuta la escena inicial; `AIGS_MAX_FRAMES` para smoke tests). |
| `exporters/` | Exportadores por plataforma (Fase 2+). *(vacío aún)* |
| `sdk/aigs-format/SPEC.md` | **Especificación normativa del formato `.aigs`** — mantener en sync con `aigs-project`. |
| `examples/hello-world/` | Proyecto `.aigs` mínimo válido; fixture del CI. |
| `tests/` | Tests de integración del sistema completo. *(vacío aún)* |

El workspace Cargo vive en la raíz (`Cargo.toml`); `editor/src-tauri` queda excluido y se compila aparte. Comandos: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`.

## Mapa de documentación (leer antes de trabajar)

| Documento | Qué contiene | Léelo cuando… |
|---|---|---|
| [docs/proyecto.md](docs/proyecto.md) | Visión completa, filosofía, objetivos, público, principios | necesites contexto general |
| [docs/plan.md](docs/plan.md) | **Plan maestro**: hitos M0–M6 del MVP con tareas, entregables y riesgos | vayas a implementar cualquier cosa |
| [docs/arquitectura.md](docs/arquitectura.md) | Módulos, formato `.aigs`, IPC editor↔runtime, **tabla de decisiones** | tomes o cuestiones decisiones técnicas |
| [docs/editor.md](docs/editor.md) | Paneles y arquitectura interna del editor | trabajes en `editor/` |
| [docs/runtime.md](docs/runtime.md) | Crates, ECS, game loop, modos de ejecución | trabajes en `runtime/` |
| [docs/ia.md](docs/ia.md) | Estrategia IA, agentes, formato AI-Ready | trabajes en integración con IA |
| [docs/exportadores.md](docs/exportadores.md) | Plataformas y diseño de exportación | trabajes en `exporters/` |
| [docs/plugins.md](docs/plugins.md) | SDK y extensibilidad | trabajes en `sdk/` |
| [docs/testing.md](docs/testing.md) | Estrategia de pruebas por nivel | escribas o revises tests |
| [docs/ci-cd.md](docs/ci-cd.md) | Pipeline, quality gates, releases | toques CI o publiques releases |
| [docs/vision.md](docs/vision.md) | Misión, visión y creencias | redactes comunicación del proyecto |

## Reglas de trabajo

1. **El plan manda:** todo trabajo se enmarca en un hito de [docs/plan.md](docs/plan.md); no adelantar alcance de fases futuras.
2. **Decisiones trazables:** toda decisión de arquitectura se registra en la tabla de [docs/arquitectura.md](docs/arquitectura.md) *antes* de implementarse.
3. **Docs sincronizadas:** si un cambio afecta a un documento de `docs/`, se actualiza en el mismo commit/PR.
4. **Formato `.aigs` es sagrado:** cualquier cambio exige actualizar la especificación en `sdk/` y las migraciones.
5. **Calidad:** Conventional Commits; `cargo fmt`/`clippy` y `prettier`/`eslint` limpios; ningún hito se cierra sin tests ([docs/testing.md](docs/testing.md)).
6. **Idioma:** documentación en español; código, identificadores y mensajes de commit en inglés.

## Trazabilidad (bitácora de hitos del desarrollo)

Registro cronológico de los pasos mayores del proyecto. Añadir una línea por cada avance significativo.

| Fecha | Avance | Referencia |
|---|---|---|
| 2026-07-03 | Redacción inicial de la documentación de diseño (11 documentos en `docs/`), sesión AI-First con Claude Code. | `docs/` |
| 2026-07-03 | Definición del plan maestro del MVP: hitos M0–M6, criterio de éxito, riesgos. | [docs/plan.md](docs/plan.md) |
| 2026-07-03 | Decisiones fundacionales: formato `.aigs` AI-Ready, ECS propio, WGPU, Tauri. | [docs/arquitectura.md](docs/arquitectura.md) |
| 2026-07-03 | Creación del monorepo (estructura de directorios, README, LICENSE MIT, CONTRIBUTING, ROADMAP, CLAUDE.md) y publicación en GitHub. | este repositorio |
| 2026-07-03 | **M0 completado:** workspace Cargo (`aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`, `aigs-cli`), especificación v0 del formato `.aigs`, ejemplo `hello-world` validado por el CLI, scaffold del editor Tauri 2 + React, CI multiplataforma en GitHub Actions. | commit `3683636`, [sdk/aigs-format/SPEC.md](sdk/aigs-format/SPEC.md) |
| 2026-07-03 | **M1 completado:** ECS con componentes/consultas/Schedule, renderer WGPU (batching instanciado, capas, cámara ortográfica), crate `aigs-runtime` (loop 60 Hz interpolado, input, runner winit), ejemplo `bouncing-sprites` verificado en GPU local. Decisiones nuevas en la tabla de arquitectura. | [docs/arquitectura.md](docs/arquitectura.md), [examples/bouncing-sprites/](examples/bouncing-sprites/) |
| 2026-07-03 | **M2 completado:** pipeline de assets (`AssetStore`), instanciación de escenas `.aigs` → `World`, comando `aigs run`, test de integración sobre `hello-world` real, sprite `width`/`height` en el formato. Corregido segfault de teardown Wayland/EGL. | [tests/integration/](tests/integration/), [sdk/aigs-format/SPEC.md](sdk/aigs-format/SPEC.md) |
| 2026-07-03 | **M3 código completo:** editor base — backend Tauri con guardado revalidado por `aigs-project`, documento con undo/redo, viewport Canvas 2D, árbol, inspector, recursos con importación, consola, Play → `aigs run`. Decisión de viewport registrada en arquitectura. Validación manual pendiente (falta `webkit2gtk-4.1` local). | [editor/](editor/), [docs/editor.md](docs/editor.md) |
