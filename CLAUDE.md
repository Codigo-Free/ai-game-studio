# CLAUDE.md — AI Game Studio

Guía de contexto para asistentes de IA que trabajen en este repositorio. Este proyecto se desarrolla con metodología **AI First**: la IA participa en diseño, código, documentación, pruebas y revisión.

## Qué es este proyecto

Plataforma **AI-First open source** para crear videojuegos 2D combinando el paradigma visual de Adobe Flash (timeline, fotogramas) con tecnología moderna (Rust, ECS, WGPU, Tauri) e IA nativa. Lema: *Build Games at the Speed of Imagination*. Principio central: **la IA conoce el videojuego y el videojuego conoce la IA** — todo el proyecto de juego vive en el formato `.aigs` (JSON AI-Ready) que editor, runtime, exportadores e IA comparten como contrato.

## Estado actual

- **Fase 2 (Motor completo) en curso** — plan en `docs/plan.md` (M7–M13). **M7–M10 completados**; siguiente: **M11 (Partículas)**.
- M10 entregó: spritesheets (`spritesheet: {frame_width, frame_height}` en el asset, `sprite.frame` animable con truncado a entero, UV rect por instancia en el shader), evento `key_released`, y **máquinas de estados de animación** (`animator: {initial, states, transitions}` — sus animaciones no se auto-reproducen; stop al salir del estado, restart al entrar; transiciones por teclas/scene_start/animation_end). Editor: frame en inspector/viewport/timeline, botón ▦ en Recursos para configurar la rejilla, sección Animator completa. Demo: robot de robot-rescue camina con spritesheet de 6 frames e idle↔walk por flechas. Recorte consciente: curvas bezier diferidas a M13.
- M9 entregó: audio con **kira 0.12** (features `cpal,wav,pcm` — ojo: `wav` solo trae el lector RIFF, sin `pcm` los WAV dan "unsupported codec"; volumen lineal→decibelios) — `AudioPlayer` en runtime (efectos `play_sound`, música por escena con `music: {asset, volume, looped}` que **continúa entre escenas si es el mismo asset**, modo deshabilitado sin dispositivo para CI/headless), acción `play_sound` en behaviors, editor con importación/pre-escucha de audio en Recursos y música en propiedades de escena, WAVs procedurales en robot-rescue (tema + pop + jingle de victoria) y physics-playground (bump en colisiones). CI/release con `libasound2-dev` en Ubuntu.
- M8 entregó: física 2D con **rapier2d 0.33** (¡API glam, no nalgebra!: `Pose::new(Vector::new(x,y), ang)`, `pipeline.step` de 12 args sin query pipeline) — componentes `rigidbody2d` (dynamic/kinematic/static, gravity_scale, velocidad inicial, fixed_rotation) y `collider2d` (box/circle, sensor, restitution, friction, tamaño default del sprite visible), `gravity` por escena (default 0,-980), `PhysicsWorld` a paso fijo (kinemáticos siguen al transform, dinámicos escriben de vuelta), evento `collision` (filtro `with`) integrado en GamePlayer, secciones de física en el inspector + gravedad en propiedades de escena (sin selección), ejemplo `examples/physics-playground` (60 FPS, 10 entidades). Limitación documentada: `move` sobre cuerpos dinámicos no altera la simulación.
- M7 entregó: `aigs export <game.aigs> [--output dir] [--zip]` con diseño **self-player** (el binario `aigs` ejecuta `data/game.aigs` si existe junto al ejecutable; exportar = copiar ejecutable renombrado + datos validados), crate `aigs-export-desktop` en `exporters/desktop` (valida antes de escribir, no sobrescribe, zip opcional), botón **⬇ Exportar** en el editor. Validado: Robot Rescue exportado a carpeta limpia corre standalone a 63 FPS.
- **Fase 1 (MVP) completada** — release **v0.1.0** (2026-07-03) con instaladores del editor y binarios del CLI para 3 SO.
- M6 entregó: benchmarks Criterion con baseline en `docs/testing.md` (query2 10k ≈ 21,5 µs), demo **Robot Rescue** (`examples/robot-rescue`: menú → nivel jugable → victoria, encadenado con `animation_end`, sprites procedurales, 100 % datos), workflow de release (`.github/workflows/release.yml`, tag `v*` → tauri-action + binarios CLI), `CHANGELOG.md`, guía de usuario `docs/guia-inicio.md` y README con estado 0.1.0.
- M5 entregó: componente `behaviors` en el formato (eventos `key_down`/`key_pressed`/`click`/`scene_start`/`animation_end` → acciones `move`/`goto_scene`/`play_animation`, documentado en SPEC), `GamePlayer` en runtime (multi-escena con `World::clear`, hit-test de clic con cámara, warnings de binding), `aigs run` reescrito sobre el player con stats por stdout (`stats: fps=… entities=…`), editor con gestión de escenas (crear/duplicar/eliminar/★ inicial en la toolbar), sección Comportamientos en el inspector (formulario "Cuando…hacer…"), y logs/métricas del player streameados a la consola del editor vía eventos Tauri (`player-log`/`player-err`). hello-world ahora es un juego: menú (clic/Enter → nivel) + nivel (flechas mueven, Esc vuelve, clic en el goal reproduce animación).
- M4 entregó: `aigs_anim::sample` (evaluación de pistas con easing; `Keyframe` unificado y reexportado por `aigs-project`), `aigs_runtime::AnimationPlayback` (bind escena→entidades con warnings, avance por tick, loop con wrap, hold del valor final), `aigs run` reproduce las animaciones de la escena, y en el editor: panel **Timeline** con pestañas inferiores (Timeline/Consola) — selector de animaciones, fps/loop, pistas por entidad+propiedad, keyframes arrastrables con editor de frame/valor/easing, doble clic para insertar, scrubbing y reproducción con preview en el viewport (`state.preview`, no undoable; viewport de solo lectura mientras hay preview). Espejo TS del muestreo en `editor/src/anim.ts` (mantener en sync con `aigs-anim`).
- M3 entregó: editor base — backend Tauri revalidando con `aigs-project`, undo/redo por snapshots, viewport Canvas 2D, árbol, inspector, recursos, consola, Play → `aigs run`.
- **Para ejecutar el editor:** `cd editor && npm run tauri dev` (`webkit2gtk-4.1` ya instalado en esta máquina). El botón Play usa el CLI `aigs`, ya instalado en `~/.cargo/bin` (`cargo install --path cli` para reinstalar tras cambios).
- Decisión clave M3: viewport de edición en Canvas 2D (misma matemática TRS que el runtime); el runtime WGPU real corre en Play vía `aigs run`. Ver tabla en `docs/arquitectura.md`.
- Deuda técnica: `wgpu` 24 / `winit` 0.30 (actualizar tras MVP); jerarquía aplanada al instanciar en runtime; docking fijo (CSS grid) sin paneles arrastrables; animación por spritesheet (frame de sprite) diferida — necesita extensión del formato (UV/frames), prevista con los spritesheets de Fase 2.
- Gotcha: en Wayland+EGL el `Renderer` debe soltarse en `exiting()` de winit o segfaultea (resuelto en `aigs-runtime/src/app.rs`).

## Mapa del repositorio

| Ruta | Contenido |
|---|---|
| `docs/` | Documentación de diseño (fuente de verdad del proyecto). |
| `editor/` | Editor visual — Tauri 2 + React + Vite + TypeScript (`npm run build` / `npm run tauri dev`). |
| `runtime/crates/` | Crates del motor: `aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`. |
| `cli/` | Binario `aigs`: `validate <game.aigs>` y `run <game.aigs>` (ejecuta la escena inicial; `AIGS_MAX_FRAMES` para smoke tests). |
| `exporters/desktop/` | `aigs-export-desktop`: exportación a carpeta auto-contenida (self-player). |
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
| 2026-07-03 | **M3 completado y validado:** editor base — backend Tauri con guardado revalidado por `aigs-project`, documento con undo/redo, viewport Canvas 2D, árbol, inspector, recursos con importación, consola, Play → `aigs run`. Validación manual del usuario: abrir hello-world, mover entidad en el viewport, guardar y ejecutar con Play. | [editor/](editor/), [docs/editor.md](docs/editor.md) |
| 2026-07-04 | **M10 completado y validado** (partida completa con el robot caminante): spritesheets con UV por instancia, sprite.frame animable, key_released, máquinas de estados de animación (animator) en formato+runtime+editor, robot caminante en robot-rescue. 18 tests runtime. Bezier diferido. | [sdk/aigs-format/SPEC.md](sdk/aigs-format/SPEC.md), [runtime/crates/aigs-runtime/src/player.rs](runtime/crates/aigs-runtime/src/player.rs) |
| 2026-07-04 | **M9 completado y validado** (partida completa menú→nivel→victoria con sonido): audio con kira — AudioPlayer (efectos + música por escena con continuidad entre escenas, no-op sin dispositivo), acción play_sound, música/pre-escucha en el editor, WAVs procedurales en las demos, ALSA en CI. | [runtime/crates/aigs-runtime/src/audio.rs](runtime/crates/aigs-runtime/src/audio.rs) |
| 2026-07-04 | **M8 completado y validado:** física 2D (rapier2d), componentes rigidbody2d/collider2d, gravedad por escena, evento collision con filtro, tests sin GPU (caída+reposo, colisión→escena, sensor), física en inspector del editor. Playground validado manualmente a 60 FPS sostenidos. | [runtime/crates/aigs-runtime/src/physics.rs](runtime/crates/aigs-runtime/src/physics.rs), [examples/physics-playground/](examples/physics-playground/) |
| 2026-07-03 | **Fase 2 planificada y M7 completado:** plan de Fase 2 (M7–M13) en docs/plan.md; exportador Desktop con diseño self-player (`aigs export`, crate `aigs-export-desktop`, botón Exportar en el editor). Robot Rescue exportado y verificado standalone (63 FPS en carpeta limpia). | [docs/plan.md](docs/plan.md), [exporters/desktop/](exporters/desktop/) |
| 2026-07-03 | **M6 completado — FASE 1 (MVP) CERRADA:** benchmarks Criterion (baseline en testing.md), demo Robot Rescue (menú+nivel+victoria, 100 % `.aigs`), CHANGELOG, guía de inicio rápido, y **release v0.1.0** publicado por el workflow de release: 7 instaladores del editor (Linux deb/rpm/AppImage, Windows msi/exe, macOS dmg/app) + 3 binarios del CLI. | [Release v0.1.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.1.0), [CHANGELOG.md](CHANGELOG.md), [docs/guia-inicio.md](docs/guia-inicio.md) |
| 2026-07-03 | **M5 completado y validado:** behaviors sin código en el formato (evento→acción), `GamePlayer` multi-escena con `goto_scene` y hit-test de clic, `aigs run` con stats, gestión de escenas y comportamientos en el editor, logs del player en la consola del editor. hello-world convertido en juego menú+nivel. Validado: round-trip de behaviors por el guardado del editor y juego ejecutado. | [sdk/aigs-format/SPEC.md](sdk/aigs-format/SPEC.md), [runtime/crates/aigs-runtime/src/player.rs](runtime/crates/aigs-runtime/src/player.rs) |
| 2026-07-03 | **M4 completado y validado:** evaluación de keyframes (`aigs_anim::sample`), `AnimationPlayback` en runtime (loop/hold, warnings de bind), `aigs run` reproduce animaciones, panel Timeline en el editor (pistas, keyframes arrastrables, easing, scrubbing, play con preview en viewport), semántica de reproducción en la SPEC. Validación manual: keyframe arrastrado en el Timeline y guardado. Spritesheet diferido a Fase 2. | [sdk/aigs-format/SPEC.md](sdk/aigs-format/SPEC.md), [editor/src/panels/Timeline.tsx](editor/src/panels/Timeline.tsx) |
