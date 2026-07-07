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

## Fase 3 — Multiplataforma 🟡 *cerrada sobre Desktop/Web/Android* ([release 0.3.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.3.0))

**Objetivo:** el mismo proyecto `.aigs`, sin cambios, exportado a Web/Android/iOS además de Desktop. Detalle en [docs/plan.md](docs/plan.md). iOS (M16) queda diferido; el resto de la fase se da por cerrada con validación pendiente en dispositivos reales.

| Hito | Alcance | Estado |
|---|---|---|
| M14 | Exportador Web (WASM): `AssetSource`, arranque de renderer asíncrono, crates `exporters/web`/`exporters/web-player`, `aigs export --target web` | 🟡 Implementado y verificado por CI/compilación — **pendiente validar en navegador real** |
| M15 | Exportador Android: `AndroidAssets`, `run_android`, entrada táctil, `virtual_button`, crates `exporters/android`/`exporters/android-player`, `aigs export --target android` | 🟡 Implementado y verificado con un `.apk` real (NDK+SDK+cargo-apk instalados en este entorno) — **pendiente validar en dispositivo/emulador real** |
| M16 | Exportador iOS | 🟤 Diferido, sin fecha (necesita macOS + Xcode, no disponibles en este entorno) |
| M17 | Menú de exportación en el editor, matriz de CI de los tres exportadores, presupuesto de tamaño (`strip`/`wasm-opt`), guía de publicación, release **0.3** | 🟢 Completado |

## Fase 4 — IA profunda 🟢 *completada*

**Objetivo:** la IA como colaborador activo — chat nativo que propone y aplica cambios sobre el `.aigs`, agentes especializados, generación de juegos completos desde lenguaje natural. Detalle en [docs/plan.md](docs/plan.md) e [docs/ia.md](docs/ia.md).

| Hito | Alcance | Estado |
|---|---|---|
| M18 | AI Core y chat con contexto del proyecto (Ollama + cloud) | 🟡 Completado y verificado con Ollama real — **proveedor Claude implementado sin verificar** (necesita API key de un usuario), recorrido de UI no verificado (sin automatización de pantalla en este entorno) |
| M19 | Escritura asistida y primer agente (Programador): propuesta de cambios (entidades + scripts) con confirmación explícita | 🟡 Completado y verificado de punta a punta con Ollama real (`qwen2.5-coder:7b`) — recorrido de UI no verificado (sin automatización de pantalla en este entorno) |
| M20 | Agentes especializados: Arquitecto, Diseñador de niveles, Programador, Física, Audio, Animador (conecta animaciones existentes) | 🟡 Completado y verificado de punta a punta con Ollama real (`qwen2.5-coder:7b`) — Optimización diferida (sin datos de perfilado aún), recorrido de UI no verificado (sin automatización de pantalla en este entorno) |
| M21 | Generación de juegos completos: un "Productor" planifica escenas (nuevas y/o la abierta), cada una construida por el motor de M20, release **0.4** | 🟡 Completado y verificado de punta a punta con Ollama real (`qwen2.5-coder:7b`) — genera **estructura** completa con assets ya importados (sin arte/audio nuevo), recorrido de UI no verificado |

## Fase 5 — Ecosistema ⚪

**Objetivo:** extensión por la comunidad sin tocar el núcleo — SDK, marketplace, colaboración en tiempo real, servicios cloud opcionales. Detalle en [docs/plan.md](docs/plan.md) e [docs/plugins.md](docs/plugins.md).

| Hito | Alcance | Estado |
|---|---|---|
| M22 | SDK de plugins v1 (paneles, componentes, importadores, exportadores) | ⚪ Pendiente |
| M23 | Marketplace (publicación, descubrimiento, instalación) | ⚪ Pendiente |
| M24 | Colaboración en tiempo real | ⚪ Pendiente |
| M25 | Servicios cloud opcionales, release **1.0** | ⚪ Pendiente |

---

Leyenda: 🔵 en curso · 🟡 implementado, validación manual pendiente · 🟢 completado · 🟤 diferido sin fecha · ⚪ pendiente
