# CLAUDE.md — AI Game Studio

Guía de contexto para asistentes de IA que trabajen en este repositorio. Este proyecto se desarrolla con metodología **AI First**: la IA participa en diseño, código, documentación, pruebas y revisión.

## Qué es este proyecto

Plataforma **AI-First open source** para crear videojuegos 2D combinando el paradigma visual de Adobe Flash (timeline, fotogramas) con tecnología moderna (Rust, ECS, WGPU, Tauri) e IA nativa. Lema: *Build Games at the Speed of Imagination*. Principio central: **la IA conoce el videojuego y el videojuego conoce la IA** — todo el proyecto de juego vive en el formato `.aigs` (JSON AI-Ready) que editor, runtime, exportadores e IA comparten como contrato.

## Estado actual

- **Fase:** 1 (MVP) — **Hito M0 (Fundaciones)** en curso.
- Hecho: documentación de diseño completa en `docs/`, estructura del monorepo, repositorio GitHub.
- Pendiente en M0: workspace Cargo en `runtime/`, proyecto Tauri+React en `editor/`, CI en GitHub Actions, especificación v0 del formato `.aigs` en `sdk/`.

## Mapa del repositorio

| Ruta | Contenido |
|---|---|
| `docs/` | Documentación de diseño (fuente de verdad del proyecto). |
| `editor/` | Editor visual — Tauri + React + TypeScript. *(vacío aún)* |
| `runtime/` | Motor — workspace Rust: `aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`. *(vacío aún)* |
| `cli/` | Herramienta CLI. *(vacío aún)* |
| `exporters/` | Exportadores por plataforma (Fase 2+). *(vacío aún)* |
| `sdk/` | Contrato público: especificación `.aigs` y APIs de extensión. *(vacío aún)* |
| `examples/` | Proyectos de ejemplo y demos. *(vacío aún)* |
| `tests/` | Tests de integración del sistema completo. *(vacío aún)* |

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
