# AI Game Studio

> **Build Games at the Speed of Imagination.**

**AI Game Studio** es una plataforma de desarrollo de videojuegos **AI-First** y **open source** que combina la productividad del paradigma visual de Adobe Flash (timeline, fotogramas, edición directa) con arquitecturas modernas (Rust, ECS, WGPU) e integra la Inteligencia Artificial como un miembro activo del equipo de desarrollo.

> **La IA conoce el videojuego y el videojuego conoce la IA.**

## Estado del proyecto

🚧 **Fase 1 — MVP en planificación.** El proyecto está en su etapa inicial de diseño y documentación. Ver el [plan maestro](docs/plan.md) y el [roadmap](ROADMAP.md).

## ¿Qué es?

- 🎨 **Editor visual** de escritorio (Linux/Windows/macOS) con escena, timeline, inspector y recursos — estilo Flash, tecnología moderna.
- ⚙️ **Runtime propio** en Rust: ECS + render 2D sobre WGPU, ligero y de alto rendimiento.
- 🤖 **IA nativa**: formato de proyecto AI-Ready que permitirá a agentes especializados crear y modificar juegos completos (Fase 4).
- 📦 **Exportación multiplataforma**: Desktop, Android, Web e iOS (Fases 2–3).
- 🧩 **Extensible**: SDK, plugins y marketplace (Fase 5).

## Estructura del monorepo

| Directorio | Contenido |
|---|---|
| [`docs/`](docs/) | Documentación de diseño: visión, arquitectura, plan, decisiones. |
| [`editor/`](editor/) | Editor visual — Tauri + React + TypeScript. |
| [`runtime/`](runtime/) | Motor de ejecución — workspace Rust (`aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`). |
| [`cli/`](cli/) | Herramienta de línea de comandos. |
| [`exporters/`](exporters/) | Exportadores por plataforma. |
| [`sdk/`](sdk/) | Contrato público: especificación del formato `.aigs` y APIs de extensión. |
| [`examples/`](examples/) | Proyectos de ejemplo y juegos de demostración. |
| [`tests/`](tests/) | Tests de integración de todo el sistema. |

## Documentación

- [Proyecto completo](docs/proyecto.md) — qué es, por qué nace, filosofía y objetivos.
- [Plan maestro](docs/plan.md) — hitos detallados del MVP (M0–M6).
- [Arquitectura](docs/arquitectura.md) — módulos, formato `.aigs`, decisiones.
- [Visión](docs/vision.md) · [Editor](docs/editor.md) · [Runtime](docs/runtime.md) · [IA](docs/ia.md) · [Exportadores](docs/exportadores.md) · [Plugins](docs/plugins.md) · [Testing](docs/testing.md) · [CI/CD](docs/ci-cd.md)

## Tecnologías

**Rust** · **WGPU** · **Tauri** · **React** · **TypeScript** · **Ollama / Claude / GPT / Gemini**

## Contribuir

El proyecto es abierto a la comunidad. Lee [CONTRIBUTING.md](CONTRIBUTING.md) para empezar.

## Licencia

[MIT](LICENSE)
