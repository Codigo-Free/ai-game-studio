# Roadmap — AI Game Studio

Resumen ejecutivo de la evolución del proyecto. El detalle completo del MVP está en [docs/plan.md](docs/plan.md).

---

## Fase 1 — MVP 🔵 *en curso*

**Objetivo:** crear un juego 2D jugable usando solo el editor, ejecutado por el runtime propio en modo Play.

| Hito | Alcance | Estado |
|---|---|---|
| M0 | Fundaciones: monorepo, CI, especificación del formato `.aigs` | 🟢 Completado |
| M1 | Núcleo del runtime: ECS, render WGPU, game loop 60 FPS | 🟢 Completado |
| M2 | Formato de proyecto y pipeline de assets | 🔵 Siguiente |
| M3 | Editor base: viewport, árbol, inspector, recursos, consola | ⚪ |
| M4 | Timeline y animación por keyframes | ⚪ |
| M5 | Escenas múltiples y modo Play | ⚪ |
| M6 | Demo, calidad y release **0.1** | ⚪ |

## Fase 2 — Motor completo ⚪

Animaciones avanzadas (curvas, máquinas de estados) · Física 2D · Audio · Partículas · Scripting de usuario · **Exportación Desktop**.

## Fase 3 — Multiplataforma ⚪

Exportadores **Android**, **Web (WASM)** e **iOS** · Optimización de rendimiento y tamaño.

## Fase 4 — IA profunda ⚪

Chat IA nativo con contexto completo del proyecto · Agentes especializados (Arquitecto, Programador, Animador, Diseñador de niveles…) · Generación automática de videojuegos a partir de lenguaje natural.

## Fase 5 — Ecosistema ⚪

SDK público de plugins · Marketplace · Servicios cloud · Trabajo colaborativo en tiempo real.

---

Leyenda: 🔵 en curso · 🟢 completado · ⚪ pendiente
