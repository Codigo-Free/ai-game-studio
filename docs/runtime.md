# Runtime

Motor de ejecución 2D en **Rust**, ligero y altamente optimizado. Ejecuta los videojuegos creados desde el editor y, en fases posteriores, es el corazón de los binarios exportados.

---

## Organización (workspace de Cargo)

| Crate | Responsabilidad |
|---|---|
| `aigs-ecs` | Entity Component System: entidades, componentes, sistemas, consultas. |
| `aigs-render` | Render 2D sobre **WGPU**: sprite batching, texturas, cámaras 2D, capas/orden Z. |
| `aigs-anim` | Animación: pistas, keyframes, interpolación (lineal + easing), evaluación por tiempo. |
| `aigs-project` | Formato `.aigs`: carga/guardado, validación por esquema, versionado y migraciones. |

---

## ECS

Arquitectura Entity Component System propia para garantizar escalabilidad, alto rendimiento, bajo acoplamiento y fácil mantenimiento.

**Componentes del MVP:** `Transform2D`, `Sprite`, `Camera2D`, `Visibility`, `Name`, `AnimationPlayer`.

Todos los componentes son serializables ↔ JSON: el estado del juego es siempre representable en el formato de proyecto (principio AI-Ready, ver [arquitectura.md](arquitectura.md)).

---

## Game loop

- Simulación a paso fijo con render interpolado; objetivo **60 FPS** estables.
- Sistema de entrada básico (teclado y ratón) en el MVP.
- Cambio de escenas en caliente (`goto_scene`) accionado por eventos.

---

## Modos de ejecución

1. **Embebido en el editor** (MVP): renderiza el viewport y ejecuta el modo Play, controlado por comandos IPC.
2. **Independiente** (Fase 2+): binario exportado que carga el proyecto empaquetado y lo ejecuta sin editor.

---

## Evolución por fases

| Fase | Capacidades del runtime |
|---|---|
| 1 (MVP) | ECS, render de sprites, animación por keyframes, escenas, entrada básica. |
| 2 | Física 2D, audio, partículas, máquinas de estados, scripting de usuario. |
| 3 | Targets Android/Web (WASM)/iOS, optimización de tamaño y rendimiento. |

---

## Rendimiento

- Benchmarks de ECS y render desde el hito M1, vigilados en CI ([testing.md](testing.md)).
- El diseño permite sustituir el ECS interno sin alterar el formato de proyecto si los benchmarks lo desaconsejan.
