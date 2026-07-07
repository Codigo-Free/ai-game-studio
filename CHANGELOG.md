# Changelog

Todos los cambios notables de AI Game Studio se documentan aquí. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/) y el versionado es [SemVer](https://semver.org/lang/es/).

## [0.4.0] — 2026-07-07 · IA profunda (Fase 4)

El chat del editor pasa de responder preguntas a proponer y aplicar cambios reales sobre el proyecto, coordinar varios agentes especializados, y generar un juego completo (varias escenas) a partir de una instrucción en lenguaje natural — siempre con confirmación explícita y deshacer de un solo paso.

### AI Core y chat de solo lectura (M18)
- `Provider` (Ollama local / Claude cloud) tras la misma interfaz; contexto del proyecto construido por el frontend a partir del estado en memoria del editor.
- Panel de **Chat** nuevo junto a Timeline/Consola.

### Escritura asistida (M19)
- Modo **"Proponer cambios"**: el modelo responde un único `ChangeProposal` (JSON) validado contra los tipos reales del formato antes de mostrarse — assets inventados, ids duplicados o scripts que no compilan se rechazan ahí mismo.
- Aplicar reutiliza el undo/redo del editor: deshacer una propuesta de la IA es `Ctrl+Z`.

### Agentes especializados (M20)
- Modo **"Orquestar agentes"**: un Arquitecto reparte una instrucción de alto nivel entre especialistas (Arquitecto, Diseñador de niveles, Programador, Física, Audio, Animador), cada uno limitado por una lista blanca de componentes comprobada en el backend.
- "Optimización" queda diferido (sin datos de perfilado todavía); "Animador" solo conecta animaciones ya existentes.

### Generación de juegos completos (M21)
- Modo **"Generar juego"**: un nuevo rol "Productor" decide qué escenas hacen falta (nuevas y/o la abierta) y cada una se construye con el mismo motor de M20. Aplicar un juego generado es un único commit de historial.
- Genera **estructura completa** (escenas, entidades, física, comportamientos, scripts) reusando assets ya importados — sin generación de arte/audio nuevo, no hay modelo de imágenes/sonido integrado.

### Verificación
- Los cuatro hitos verificados de punta a punta contra un Ollama real (`qwen2.5-coder:7b`), incluido un caso de estudio de generación de un mini-juego de dos escenas.
- Recorrido de clics en la UI real no verificado (sin automatización de pantalla en este entorno de desarrollo); proveedor Claude implementado pero no verificado en vivo (necesita una API key propia).

## [0.3.0] — 2026-07-06 · Multiplataforma (Fase 3, parcial)

Un mismo proyecto `.aigs`, sin cambios, corre en Desktop, en el navegador y en Android. iOS queda diferido (necesita macOS + Xcode, no disponibles en este ciclo de desarrollo).

### Exportador Web — WASM (M14)
- `aigs export --target web`: WebGPU/WebGL vía WGPU, sin cambios en el motor.
- Jugador genérico (`aigs_web_player.js`/`_bg.wasm`) que hace `fetch` del proyecto en tiempo de ejecución — mismo diseño self-player que Desktop.
- Carga de assets (`aigs_runtime::AssetSource`) abstraída para leer de disco (Desktop) o de memoria prellenada por `fetch` (Web), sin duplicar el pipeline de decodificación.

### Exportador Android — APK firmado (M15)
- `aigs export --target android [--release]`: compila y firma un APK real (Vulkan vía WGPU) con `cargo apk build`.
- Entrada táctil (un dedo = ratón) y nuevo componente de formato `virtual_button` para controles estilo teclado en pantalla.
- A diferencia de Web, el jugador de Android es una plantilla de build (los assets se empaquetan dentro del APK en tiempo de compilación), no un artefacto único reutilizable.

### Editor
- El botón **⬇ Exportar** gana un desplegable Desktop/Web/Android.

### Presupuesto de tamaño (M17)
- `opt-level = "s"` + `lto = true` + `strip = true` en el perfil de release de ambos jugadores; `wasm-opt -Oz` sobre el `.wasm` en CI.

### Documentación
- `docs/guia-publicacion.md`: pasos manuales para publicar en Web (GitHub Pages/hosting estático) y Android (Google Play, keystore de release propio).

### Limitaciones conocidas
- Sin `save.json` en Web ni Android todavía (no hay sistema de archivos accesible de la misma forma que en Desktop).
- Validación manual en un navegador y un dispositivo/emulador Android reales todavía pendiente — verificado en este ciclo por compilación, enlace y empaquetado reales, no jugando de verdad.
- M16 (iOS) diferido sin fecha.
- Firmar un release de Android (keystore propio) sigue siendo solo CLI, sin UI en el editor.

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
