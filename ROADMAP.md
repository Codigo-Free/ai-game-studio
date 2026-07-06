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

## Fase 2 — Motor completo 🟢 *completada* ([release 0.2.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.2.0))

**Objetivo:** juegos distribuibles y con sensación de juego real. Detalle en [docs/plan.md](docs/plan.md).

| Hito | Alcance | Estado |
|---|---|---|
| M7 | Exportador Desktop (`aigs export`, binario auto-contenido) | 🟢 Completado |
| M8 | Física 2D (rapier2d, colisiones → behaviors) | 🟢 Completado |
| M9 | Audio (efectos, música, acción `play_sound`) | 🟢 Completado |
| M10 | Spritesheets y máquinas de estados de animación | 🟢 Completado (bezier diferido, sin fecha) |
| M11 | Partículas | 🟢 Completado |
| M12 | Scripting de usuario (rhai) | 🟢 Completado |
| M13 | Persistencia de scripts + demo **Tamagotchi**, release **0.2** (reemplaza al platformer original) | 🟢 Completado (validado manualmente) |

## Fase 3 — Multiplataforma ⚪

**Objetivo:** el mismo proyecto `.aigs`, sin cambios, exportado a Web/Android/iOS además de Desktop. Detalle en [docs/plan.md](docs/plan.md).

| Hito | Alcance | Estado |
|---|---|---|
| M14 | Exportador Web (WASM) | ⚪ Pendiente |
| M15 | Exportador Android | ⚪ Pendiente |
| M16 | Exportador iOS | ⚪ Pendiente |
| M17 | Optimización, paridad y publicación, release **0.3** | ⚪ Pendiente |

## Fase 4 — IA profunda ⚪

**Objetivo:** la IA como colaborador activo — chat nativo que propone y aplica cambios sobre el `.aigs`, agentes especializados, generación de juegos completos desde lenguaje natural. Detalle en [docs/plan.md](docs/plan.md) e [docs/ia.md](docs/ia.md).

| Hito | Alcance | Estado |
|---|---|---|
| M18 | AI Core y chat con contexto del proyecto (Ollama + cloud) | ⚪ Pendiente |
| M19 | Escritura asistida y primer agente (Programador) | ⚪ Pendiente |
| M20 | Agentes especializados (Arquitecto, Animador, Niveles, Audio, Física, Optimización) | ⚪ Pendiente |
| M21 | Generación de juegos completos de punta a punta, release **0.4** | ⚪ Pendiente |

## Fase 5 — Ecosistema ⚪

**Objetivo:** extensión por la comunidad sin tocar el núcleo — SDK, marketplace, colaboración en tiempo real, servicios cloud opcionales. Detalle en [docs/plan.md](docs/plan.md) e [docs/plugins.md](docs/plugins.md).

| Hito | Alcance | Estado |
|---|---|---|
| M22 | SDK de plugins v1 (paneles, componentes, importadores, exportadores) | ⚪ Pendiente |
| M23 | Marketplace (publicación, descubrimiento, instalación) | ⚪ Pendiente |
| M24 | Colaboración en tiempo real | ⚪ Pendiente |
| M25 | Servicios cloud opcionales, release **1.0** | ⚪ Pendiente |

---

Leyenda: 🔵 en curso · 🟢 completado · ⚪ pendiente
