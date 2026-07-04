# Changelog

Todos los cambios notables de AI Game Studio se documentan aquí. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/) y el versionado es [SemVer](https://semver.org/lang/es/).

## [0.1.0] — 2026-07-03 · MVP (Fase 1)

Primera versión pública: el ciclo completo **crear → animar → jugar** funciona de punta a punta, sin escribir código.

### Editor visual (Tauri 2 + React)
- Viewport Canvas 2D con selección, arrastre, zoom al cursor, pan, drop de assets y marcadores de cámara.
- Árbol de escena (crear, renombrar, eliminar, reordenar, hijos) e inspector de componentes.
- **Timeline estilo Flash**: pistas por entidad/propiedad, keyframes arrastrables con easing, scrubbing y reproducción con preview en vivo.
- Gestión de escenas: crear, duplicar, eliminar, escena inicial (★).
- Comportamientos sin código desde el inspector: "Cuando [evento] hacer [acción]".
- Panel de recursos con importación y miniaturas; consola con logs y métricas del player en vivo.
- Undo/redo global (Ctrl+Z/Ctrl+Shift+Z), guardado revalidado por la implementación de referencia del formato.

### Runtime (Rust + WGPU)
- ECS propio con índices generacionales y consultas multi-componente (10k entidades consultadas en ~21 µs).
- Render 2D instanciado con orden por capas y cámara ortográfica; 60 FPS estables.
- Game loop a paso fijo (60 Hz) con render interpolado.
- Reproducción de animaciones por keyframes (loop/hold, easing) y **motor de comportamientos**: teclas, clic con hit-test, inicio de escena y fin de animación → mover, cambiar de escena, reproducir animación.
- Multi-escena con `goto_scene`.

### Formato `.aigs` v0 (AI-Ready)
- JSON legible y autodescriptivo: manifiesto, escenas, entidades, componentes, animaciones y comportamientos.
- Especificación normativa en `sdk/aigs-format/SPEC.md`; componentes de plugin preservados en round-trip.

### Herramientas
- CLI `aigs`: `validate` (manifiesto, escenas y assets) y `run` (ejecuta el juego con stats).
- CI multiplataforma (Linux/Windows/macOS) con tests, clippy, benchmarks y validación de ejemplos.

### Ejemplos
- **Robot Rescue**: juego demo con menú, nivel jugable y pantalla de victoria — 100 % datos `.aigs`.
- **hello-world**: proyecto mínimo con animaciones y comportamientos.

### Limitaciones conocidas
- Sin física, audio ni partículas (Fase 2). Sin exportación de binarios independientes (Fase 2). Sin spritesheets (Fase 2). El chat IA integrado llega en la Fase 4.
