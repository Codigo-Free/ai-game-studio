# Roadmap — AI Game Studio

Resumen ejecutivo de la evolución del proyecto. El detalle completo del MVP está en [docs/plan.md](docs/plan.md).

---

## Fase 1 — MVP 🟢 *completada* ([release 0.1.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.1.0))

**Objetivo:** crear un juego 2D jugable usando solo el editor, ejecutado por el runtime propio en modo Play. ✔ Demostrado con [Robot Rescue](examples/robot-rescue/).

| Hito | Alcance | Estado |
|---|---|---|
| M0 | Fundaciones: monorepo, CI, especificación del formato `.aigs` | 🟢 Completado |
| M1 | Núcleo del runtime: ECS, render WGPU, game loop 60 FPS | 🟢 Completado |
| M2 | Formato de proyecto y pipeline de assets | 🟢 Completado |
| M3 | Editor base: viewport, árbol, inspector, recursos, consola | 🟢 Completado (validado manualmente) |
| M4 | Timeline y animación por keyframes | 🟢 Completado (validado manualmente) |
| M5 | Escenas múltiples y modo Play | 🟢 Completado (validado manualmente) |
| M6 | Demo, calidad y release **0.1** | 🟢 Completado — v0.1.0 publicado con instaladores para 3 SO |

## Fase 2 — Motor completo 🔵 *en curso*

**Objetivo:** juegos distribuibles y con sensación de juego real. Detalle en [docs/plan.md](docs/plan.md).

| Hito | Alcance | Estado |
|---|---|---|
| M7 | Exportador Desktop (`aigs export`, binario auto-contenido) | 🟢 Completado |
| M8 | Física 2D (rapier2d, colisiones → behaviors) | 🔵 Siguiente |
| M9 | Audio (efectos, música, acción `play_sound`) | ⚪ |
| M10 | Spritesheets, curvas y máquinas de estados de animación | ⚪ |
| M11 | Partículas | ⚪ |
| M12 | Scripting de usuario (rhai) | ⚪ |
| M13 | Demo de plataformas y release **0.2** | ⚪ |

## Fase 3 — Multiplataforma ⚪

Exportadores **Android**, **Web (WASM)** e **iOS** · Optimización de rendimiento y tamaño.

## Fase 4 — IA profunda ⚪

Chat IA nativo con contexto completo del proyecto · Agentes especializados (Arquitecto, Programador, Animador, Diseñador de niveles…) · Generación automática de videojuegos a partir de lenguaje natural.

## Fase 5 — Ecosistema ⚪

SDK público de plugins · Marketplace · Servicios cloud · Trabajo colaborativo en tiempo real.

---

Leyenda: 🔵 en curso · 🟢 completado · ⚪ pendiente
