# Changelog

Todos los cambios notables de AI Game Studio se documentan aquí. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/) y el versionado es [SemVer](https://semver.org/lang/es/).

## [0.2.0] — 2026-07-06 · Motor completo (Fase 2)

Un juego hecho con AI Game Studio ya se siente como un juego de verdad —física, sonido, spritesheets, partículas, scripting y persistencia— y se puede exportar como binario independiente.

### Exportación (M7)
- `aigs export <game.aigs> [--output dir] [--zip]`: carpeta autocontenida y distribuible, diseño *self-player* (el mismo binario `aigs` corre como juego si encuentra `data/game.aigs` junto a él).
- Botón **⬇ Exportar** en el editor.

### Física 2D (M8)
- Motor `rapier2d`: componentes `rigidbody2d` (dinámico/kinemático/estático) y `collider2d` (caja/círculo, sensor), gravedad por escena.
- Evento de comportamiento `collision` con filtro por entidad.

### Audio (M9)
- Motor `kira`: efectos (`play_sound`) y música por escena con continuidad entre escenas si el asset no cambia; modo sin dispositivo para CI/headless.
- Importación y pre-escucha de audio en el editor.

### Spritesheets y animación avanzada (M10)
- Sprites animados por spritesheet (`sprite.frame`, UV por instancia) y **máquinas de estados de animación** (`animator`: estados + transiciones por teclas/eventos).

### Partículas (M11)
- Componente emisor simulado como entidades ECS (emisión continua o en ráfaga, fade/shrink); vista previa en vivo en el inspector.

### Scripting de usuario (M12)
- Lenguaje embebido **rhai**, sandboxeado (sin IO, límite de operaciones por tick): lifecycle `on_start`/`on_update`/`on_collision`/`on_destroy`, estado persistente por instancia vía `get_var`/`set_var`.
- Manifiesto tipado de la API (`aigs script-api`, `sdk/aigs-format/scripting-api.json`) y **hot reload** de scripts sin reiniciar Play mode.

### Persistencia entre partidas y demo Tamagotchi (M13)
- `save.json` (fuera del formato `.aigs`, estado de partida): memoria de scripts persistida entre escenas y **entre sesiones reales** del juego, con `elapsed_since_save()` de consumo único para modelar el tiempo transcurrido mientras el juego estaba cerrado.
- `aigs run` carga el guardado al iniciar y autoguarda periódicamente.
- **Ejemplo nuevo — Tamagotchi**: mascota con hambre/felicidad/salud que decaen en tiempo real incluso con el juego cerrado, cuidada con teclado, un único script.

### Herramientas
- `aigs script-api`: manifiesto JSON de la API de scripting, para IA y tooling de editor.

### Limitaciones conocidas
- `move` sobre cuerpos físicos dinámicos no altera la simulación (usar cinemáticos).
- Curvas de easing personalizadas (bezier), diferidas sin fecha desde M10.
- Sin gancho de cierre limpio para el autoguardado (pérdida máxima: la ventana entre autoguardados).
- Sin exportación cruzada de plataforma todavía (llega en Fase 3).

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
