# Plan Maestro — AI Game Studio

Este documento define el plan de desarrollo del proyecto, fase por fase e hito por hito. El detalle es más fino cuanto más cercana está la fase (Fase 1 y 2, ya trabajadas hito a hito) y se afinará en las fases 3–5 a medida que se alcancen. La visión general del producto está en [proyecto.md](proyecto.md).

---

## Fases del proyecto

| Fase | Nombre | Alcance | Estado |
|---|---|---|---|
| **1** | MVP | Editor visual, Timeline, Escenas, Assets, Runtime básico | 🟢 **Completada** — [release 0.1.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.1.0) (estado por hito en [ROADMAP.md](../ROADMAP.md)) |
| **2** | Motor completo | Animaciones avanzadas, Física, Audio, Partículas, Exportación Desktop | 🟢 **Completada** — [release 0.2.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.2.0) |
| **3** | Multiplataforma | M14–M17: exportadores Web (WASM), Android, iOS, optimización y publicación | 🔵 En curso — M14 (Web) y M15 (Android) implementados, pendiente de validación manual en navegador/dispositivo real |
| **4** | IA profunda | M18–M21: AI Core y chat, escritura asistida, agentes especializados, generación de juegos completos | ⚪ Pendiente |
| **5** | Ecosistema | M22–M25: SDK de plugins, marketplace, colaboración en tiempo real, servicios cloud opcionales | ⚪ Pendiente |

---

# Fase 1 — MVP

## Objetivo del MVP

Un editor de escritorio (Linux/Windows/macOS) donde un usuario puede:

1. Crear un proyecto de videojuego 2D.
2. Importar sprites y organizarlos en un panel de recursos.
3. Componer escenas arrastrando objetos al lienzo y editando sus propiedades en el inspector.
4. Animar objetos mediante una línea de tiempo con fotogramas clave y tweens.
5. Organizar el juego en múltiples escenas y navegar entre ellas.
6. Presionar **Play** y ver el juego ejecutándose dentro del editor sobre el runtime propio.

**Criterio de éxito:** construir un juego de demostración (personaje que se mueve por un escenario con animaciones y cambio de escena) usando únicamente el editor, ejecutándose a 60 FPS estables.

### Dentro del alcance del MVP

- Editor Tauri + React + TypeScript con paneles acoplables.
- Runtime Rust con ECS propio y render 2D sobre WGPU.
- Formato de proyecto **AI-Ready**: JSON legible, autodescriptivo y documentado, diseñado desde el día uno para que un modelo de IA pueda leerlo y generarlo (base de la Fase 4).
- Sistema de escenas y sistema de assets (sprites e imágenes).
- Timeline con keyframes, tweens de interpolación lineal y easing básico.
- Modo Play embebido en el editor (preview).
- Entrada de teclado/ratón básica en el preview.

### Fuera del alcance del MVP (fases posteriores)

- Física, audio, partículas → Fase 2.
- Exportación de binarios (Desktop/Android/Web/iOS) → Fases 2–3.
- Scripting de usuario y máquinas de estados → Fase 2.
- Chat IA y agentes → Fase 4 (el MVP solo prepara el formato y los puntos de extensión).
- Plugins y marketplace → Fase 5.

---

## Hitos del MVP

### M0 — Fundaciones

Preparar el terreno técnico y de proyecto.

**Tareas**
- Inicializar repositorio Git y estructura del monorepo (`editor/`, `runtime/`, `cli/`, `exporters/`, `sdk/`, `examples/`, `tests/`, `docs/`).
- Configurar workspace de Cargo (crates del runtime) y proyecto Node (editor Tauri + React + Vite + TypeScript).
- Configurar CI en GitHub Actions: build + lint + tests en cada push ([ci-cd.md](ci-cd.md)).
- Escribir la especificación v0 del formato de proyecto `.aigs` (JSON): proyecto, escenas, entidades, componentes, assets, animaciones.
- README, LICENSE, CONTRIBUTING.

**Entregable:** repositorio compilable de extremo a extremo con CI verde y RFC del formato de proyecto aprobado.

---

### M1 — Núcleo del runtime

El motor mínimo que dibuja algo en pantalla.

**Tareas**
- Crate `aigs-ecs`: entidades, componentes, sistemas, consultas.
- Componentes base: `Transform2D`, `Sprite`, `Camera2D`, `Visibility`, `Name`.
- Crate `aigs-render`: inicialización WGPU, sprite batching, texturas, capas/orden Z.
- Game loop con tiempo fijo de simulación y render interpolado (60 FPS objetivo).
- Sistema de entrada básico (teclado y ratón).
- Ejemplo mínimo en `examples/`: sprites moviéndose renderizados por el runtime, sin editor.

**Entregable:** binario de ejemplo que renderiza cientos de sprites a 60 FPS en Linux, Windows y macOS.

---

### M2 — Formato de proyecto y assets

El proyecto como datos que humanos, editor e IA comparten.

**Tareas**
- Crate `aigs-project`: carga/guardado del formato `.aigs`, validación con esquema, versionado del formato.
- Serialización de escenas: entidades y componentes ↔ JSON.
- Pipeline de assets: importación de imágenes (PNG/JPG), catálogo de assets con metadatos, rutas relativas al proyecto.
- Carga de escenas en el runtime desde el formato de proyecto.
- Documentar el formato en el SDK (`sdk/`) como contrato público.

**Entregable:** el ejemplo de M1 reescrito como datos: el runtime carga y ejecuta una escena definida enteramente en archivos `.aigs`.

---

### M3 — Editor base

La aplicación visual sobre la que se construye todo lo demás.

**Tareas**
- Shell Tauri: ventana principal, menús, apertura/creación/guardado de proyectos.
- Layout de paneles acoplables (docking) en React.
- **Viewport de escena**: render de la escena vía runtime embebido, con selección, movimiento, zoom y pan.
- **Árbol de objetos**: jerarquía de entidades de la escena (crear, renombrar, eliminar, reordenar).
- **Inspector**: edición de propiedades de los componentes de la entidad seleccionada.
- **Panel de recursos**: importación por arrastre, miniaturas, arrastrar sprite → escena.
- **Consola**: logs del editor y del runtime.
- Undo/redo global sobre el modelo de documento.
- Comunicación editor ↔ runtime (comandos y eventos vía IPC de Tauri).

**Entregable:** el usuario compone una escena visualmente y el archivo `.aigs` resultante se ejecuta en el runtime.

---

### M4 — Timeline y animación

El corazón "Flash" del producto.

**Tareas**
- Crate `aigs-anim`: pistas de animación, keyframes, interpolación (lineal + easing básico), evaluación por tiempo.
- Panel **Timeline** en el editor: capas por entidad, fotogramas, creación/edición/arrastre de keyframes, scrubbing.
- Propiedades animables del MVP: posición, rotación, escala, opacidad, frame de sprite (animación por spritesheet).
- Reproducción de animaciones en el viewport (play/pausa/loop desde el timeline).
- Persistencia de animaciones en el formato `.aigs`.

**Entregable:** animar un personaje con keyframes desde el editor y reproducirlo en el viewport.

---

### M5 — Escenas y modo Play

Del lienzo estático al juego jugable.

**Tareas**
- Gestión de múltiples escenas: crear, duplicar, eliminar, escena inicial.
- API de cambio de escena en el runtime (`goto_scene`) accionable por eventos simples (ej. tecla, clic, fin de animación).
- **Modo Play** en el editor: ejecutar/pausar/detener el juego en el viewport sin corromper el estado de edición.
- Eventos y acciones básicas sin código (estilo comportamientos): "al presionar tecla → mover", "al hacer clic → ir a escena".
- Métricas en la consola: FPS, entidades, tiempo de frame.

**Entregable:** un juego con menú y un nivel navegables desde el modo Play.

---

### M6 — Demo, calidad y release 0.1

Cerrar el MVP con un resultado demostrable.

**Tareas**
- Juego de demostración completo en `examples/` construido solo con el editor.
- Suite de tests: unitarios (ECS, animación, formato), integración (cargar proyecto → ejecutar escena) y benchmark de render ([testing.md](testing.md)).
- Empaquetado del editor para Linux, Windows y macOS (instaladores vía CI).
- Documentación de usuario: guía de inicio rápido y tour del editor.
- *Stretch goal (preparación Fase 4):* panel de Chat IA experimental conectado a Ollama con acceso de solo lectura al contexto del proyecto.

**Entregable:** **AI Game Studio 0.1** publicado en GitHub Releases con demo, documentación y binarios firmados por CI.

---

## Arquitectura del MVP

Detalle completo en [arquitectura.md](arquitectura.md).

```
┌────────────────────────── Editor (Tauri) ──────────────────────────┐
│  React + TypeScript                                                │
│  Escena · Árbol · Inspector · Recursos · Timeline · Consola        │
└───────────────▲────────────────────────────────▲───────────────────┘
                │ IPC (comandos/eventos)         │
┌───────────────▼───────────────┐   ┌────────────▼────────────┐
│        Runtime (Rust)         │   │   Proyecto .aigs (JSON) │
│  aigs-ecs · aigs-render(WGPU) │◄──│  escenas · entidades ·  │
│  aigs-anim · aigs-project     │   │  assets · animaciones   │
└───────────────────────────────┘   └─────────────────────────┘
```

**Decisión clave — formato AI-Ready:** todo lo que el editor puede hacer se expresa como datos en `.aigs`. La IA de la Fase 4 no manipulará la interfaz: leerá y escribirá el mismo formato que el editor, por lo que su calidad depende de que este contrato sea claro desde el MVP.

---

## Calidad (transversal a todos los hitos)

- **Tests** unitarios y de integración desde M1; ningún hito se cierra sin tests de su alcance.
- **CI/CD** desde M0: build multiplataforma, clippy + rustfmt, eslint + prettier, tests.
- **Benchmarks** de render y ECS desde M1, vigilados en CI.
- **Documentación**: cada hito actualiza los documentos de `docs/` afectados.

---

## Flujo de desarrollo AI First

El propio desarrollo del proyecto usa la metodología que el producto predica:

- **HarnessOS** + **Visual Studio Code** + **Git/GitHub**.
- Modelos locales (**Ollama**) y cloud (**Claude, GPT, Gemini**) para diseño, generación de código, documentación, pruebas, revisión y refactorización.
- Agentes especializados para tareas repetitivas (generación de ejemplos, cobertura de tests, changelog).
- Toda decisión de arquitectura relevante se registra en `docs/` antes de implementarse.

---

## Riesgos principales

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Integración viewport editor ↔ runtime WGPU (render embebido en Tauri) | Alto | Prototipo técnico al inicio de M3; alternativa: render a textura compartida o canal de streaming de frames. |
| Alcance del timeline crece hacia "Flash completo" | Alto | MVP limitado a keyframes + tween lineal/easing sobre 5 propiedades; el resto va a Fase 2. |
| Formato `.aigs` inestable rompe proyectos | Medio | Versionado del formato + migraciones desde M2. |
| Rendimiento del ECS propio insuficiente | Medio | Benchmarks desde M1; el diseño permite sustituir el ECS interno sin tocar el formato. |

---

# Fase 2 — Motor completo

## Objetivo de la Fase 2

Que un juego hecho con AI Game Studio pueda **distribuirse** (binario independiente) y **sentirse como un juego de verdad**: física, sonido, spritesheets y efectos. Culmina en la **release 0.2** con una demo de plataformas.

**Criterio de éxito:** un juego de plataformas con física, sonido y animación por spritesheet, creado en el editor y exportado como ejecutable que corre en una máquina sin AI Game Studio instalado.

## Hitos de la Fase 2

### M7 — Exportador Desktop
El primer binario de juego independiente del editor.
- `aigs export <game.aigs> [--output dir] [--zip]`: genera una carpeta distribuible `NombreDelJuego/` con el ejecutable + `data/` (proyecto y assets empaquetados).
- Diseño *self-player*: el propio binario `aigs` detecta al arrancar si existe `data/game.aigs` junto al ejecutable y se comporta como player (exportar = copiar el ejecutable renombrado + datos). Un solo binario, cero dependencias de compilación para el usuario.
- Botón **Exportar** en el editor; interfaz común de exportadores en `exporters/` para las plataformas de Fase 3.
- **Entregable:** Robot Rescue exportado corre en una carpeta limpia, sin repo ni CLI instalados.

### M8 — Física 2D
- Integración de un motor de física (candidato: `rapier2d`; decisión registrada en arquitectura).
- Componentes de formato: `rigidbody2d` (dinámico/kinemático/estático, gravedad, velocidad), `collider2d` (caja/círculo, sensor).
- Evento de comportamiento `collision` (con filtro por entidad/etiqueta) → acciones existentes.
- Editor: sección física en el inspector; gravedad del proyecto en configuración.
- **Entregable:** cajas que caen, rebotan y disparan eventos al chocar, sin código.

### M9 — Audio
- Motor de audio (candidato: `kira`): efectos y música con volumen y loop.
- Formato: assets `audio` (ya reservados en v0), acción `play_sound`, música de escena en la cabecera de escena.
- Editor: importación y pre-escucha en Recursos; acción de sonido en comportamientos.
- **Entregable:** la demo suena — música de fondo y efectos al interactuar.

### M10 — Spritesheets y animación avanzada
- Formato: metadatos de spritesheet en el asset (tamaño de frame, filas/columnas), propiedad animable `sprite.frame`; UV por frame en el renderer.
- Curvas de easing personalizadas (bezier) y **máquinas de estados de animación** (estados + transiciones por condición/evento).
- Editor: selector de frame en el inspector, pista de frames en el Timeline.
- **Entregable:** personaje que camina con spritesheet y cambia de estado idle↔walk.

### M11 — Partículas
- Componente emisor (tasa, vida, velocidad/dispersión, escala/opacidad en el tiempo) evaluado en el runtime con render instanciado.
- Editor: panel de partículas con vista previa.
- **Entregable:** explosiones y polvo de salto en la demo.

### M12 — Scripting de usuario
- Lenguaje embebido (candidato: `rhai`, Rust puro) como escape natural cuando los behaviors se quedan cortos.
- API mínima: acceso a entidades/componentes, entrada, escenas y animaciones; scripts como assets referenciados por un componente `script`.
- Editor: asset de script + errores en consola.
- **Entregable:** un enemigo patrulla con un script de 20 líneas.

### M13 — Persistencia y demo Tamagotchi, release 0.2

**Cambio de alcance (2026-07-06):** el plan original de M13 era una demo de plataformas. A petición del usuario, la demo de cierre de Fase 2 pasa a ser un **Tamagotchi persistente en el tiempo** — no necesita física ni plataformas, pero exige una capacidad de motor que no existía: guardar y recuperar el estado de los scripts entre sesiones, con el tiempo real transcurrido mientras el juego estaba cerrado. Las curvas bezier (diferidas desde M10) quedan diferidas de nuevo, sin fecha; no son necesarias para esta demo.

**Sistema de persistencia (nuevo, decisión en `docs/arquitectura.md`):**
- `save.json` junto al `game.aigs` del proyecto (o de los datos exportados): `{ version, saved_at_unix, scripts: { <id de entidad>: { <variable>: valor } } }`. No forma parte del formato `.aigs` (es estado de partida del jugador, no datos de diseño).
- La memoria de `get_var`/`set_var` de cada script pasa a sobrevivir también **al cambiar de escena dentro de una misma sesión** (antes se perdía al salir de la escena) — se conserva por id de entidad autor en `ScriptHost`, independiente del ciclo de vida de la escena.
- Nueva función de scripting `elapsed_since_save()`: segundos reales desde el último guardado, **consumible una sola vez** por sesión (la primera llamada de cualquier script la devuelve; el resto reciben `0`) — así el script de la mascota puede adelantar hambre/ánimo analíticamente sin importar en qué escena o entidad se consulte.
- Autoguardado periódico (no al cerrar la ventana, para no añadir un hook de cierre limpio todavía); pérdida máxima aceptada: la ventana entre autoguardados.
- Limitación documentada: el archivo de guardado vive junto a los datos del proyecto (no en un directorio de perfil de usuario); válido para el MVP de esta demo, revisar si se generaliza a más juegos.

**Tareas:**
- `aigs-runtime`: `SaveData` (serde), `ScriptHost::persisted_memory` entre escenas, `elapsed_since_save()` en la API de scripting (manifiesto + JSON regenerado), tests sin GPU (round-trip, segundos offline, memoria que sobrevive un cambio de escena).
- `GamePlayer`: cargar/exportar el estado de scripts; inyectar `offline_seconds` antes del primer `bind`.
- CLI (`aigs run`): cargar `save.json` al iniciar si existe, autoguardado cada ~10 s.
- Demo `examples/tamagotchi/`: mascota con stats (hambre/ánimo/edad) que decaen con `on_update(dt)`, cambios de estado por spritesheet/`play_animation`, controles por teclado (alimentar/jugar/limpiar/medicina — un icono clicable que module el estado de *otra* entidad no está soportado por la API de scripting v0; se documenta como limitación conocida), sonidos de interacción, fin de partida por abandono.
- Benchmarks nuevos si aplica, CHANGELOG y **release 0.2** con instaladores + demo exportada como artefacto.

## Riesgos de la Fase 2

| Riesgo | Impacto | Mitigación |
|---|---|---|
| La física introduce no-determinismo o rompe el paso fijo | Alto | rapier con timestep fijo (ya lo usamos); tests de regresión de simulación. |
| Crecimiento del formato sin control (v0 → v1) | Alto | Cada hito que toque el formato actualiza SPEC + migración en `aigs-project` en el mismo PR. |
| Audio multiplataforma (backends del SO) | Medio | `kira`/`cpal` abstraen; smoke test por SO en CI (sin dispositivo: modo dummy). |
| Scripting abre superficie de seguridad/estabilidad | Medio | `rhai` sandboxeado (sin IO), límites de operaciones por tick. |

---


# Fase 3 — Multiplataforma

## Objetivo de la Fase 3

El mismo proyecto `.aigs`, sin cambios, corriendo como aplicación nativa en **Android** e **iOS** y como página web (**WASM**). La Fase 2 ya deja el runtime completo (física, audio, partículas, spritesheets, scripting); la Fase 3 no añade funcionalidad de juego — lleva ese mismo motor a más pantallas y lo hace lo bastante ligero y rápido para móvil y navegador.

**Criterio de éxito:** exportar el mismo juego (p. ej. Robot Rescue o el Tamagotchi) a Desktop, Web, Android e iOS desde el mismo `game.aigs`, sin tocar el proyecto, y que los cuatro se sientan fluidos en su plataforma.

### Dentro del alcance

- Exportadores Web, Android e iOS sobre la interfaz común de `exporters/` ya establecida en M7.
- Adaptación de entrada táctil (tap/drag) donde hoy solo hay teclado/ratón — sin romper los proyectos existentes.
- Presupuesto de tamaño y rendimiento por plataforma (el juego debe caber y arrancar rápido en un móvil de gama media).

### Fuera del alcance (fases posteriores)

- Cualquier funcionalidad nueva de motor (eso ya cerró en Fase 2) — Fase 3 es puramente de distribución.
- Publicación real en tiendas (Play Store/App Store) a nombre del proyecto: se documenta el camino, no se gestiona la cuenta de desarrollador de cada usuario.

## Hitos de la Fase 3

### M14 — Exportador Web (WASM)

El de menor riesgo técnico: `aigs-render` ya usa WGPU, que compila a WebGPU/WebGL vía `wgpu` sin reescribir el renderer — confirmado compilando de verdad antes de comprometer el diseño (ver decisiones en `docs/arquitectura.md`).

**Entregado:**
- `runtime/crates/aigs-runtime/src/source.rs`: trait `AssetSource` (Desktop = `PathBuf` sobre `std::fs`; Web = `MemoryAssets`, un `HashMap` prellenado por `fetch`) del que ahora leen `AssetStore`, `AudioPlayer` y `ScriptHost` — mismo código de parseo/decodificación en ambas plataformas, solo cambia de dónde vienen los bytes.
- `aigs_runtime::app`: arranque del renderer como máquina de estados (`Renderer::new` bloqueante en Desktop; `Renderer::new_async` + `Rc<RefCell<Option<...>>>` resuelto de forma perezosa en Web, porque `pollster::block_on` bloquearía el único hilo de JS del navegador). El bucle de eventos usa `run_app` (Desktop, bloqueante) o `spawn_app` (Web, no bloqueante).
- Entrada de teclado/ratón: **sin cambios** — winit ya entrega los mismos `WindowEvent` en ambas plataformas, así que behaviors y scripts existentes funcionan igual sin tocarlos.
- Hot reload de scripts (M12) queda inerte en Web sin código especial: `AssetSource::as_path()` devuelve `None` para `MemoryAssets`, así que `ScriptHost` nunca registra rutas que vigilar.
- Nuevo crate `exporters/web-player` (`aigs-web-player`, cdylib wasm32, excluido del workspace nativo): el jugador genérico — `fetch`ea `data/game.aigs` y todo lo que referencia, y corre exactamente el mismo `GamePlayer` que `aigs run`.
- Nuevo crate `exporters/web` (`aigs-export-web`, sí en el workspace): empaqueta `data/` + el jugador prebuilt + un `index.html` generado. `aigs export --target web` — el jugador se busca en `<carpeta del ejecutable>/web-player/`, construido una vez con `wasm-bindgen` (ver job *Web player* en CI), igual de "cero compilación para quien exporta" que Desktop.
- Verificado de punta a punta en este entorno: `cargo build --target wasm32-unknown-unknown` limpio (lint + build), `wasm-bindgen` produce un módulo JS válido, y `aigs export --target web` sobre Robot Rescue genera la carpeta esperada (`index.html` + `.wasm`/`.js` + `data/`).

**Pendiente / limitaciones conocidas:**
- **Validación manual en un navegador real todavía no hecha** (no hay navegador disponible en este entorno de desarrollo) — falta confirmar renderizado, entrada y audio jugando de verdad antes de marcar el hito como validado.
- Sin `save.json` en Web todavía (sin sistema de archivos) — el Tamagotchi de M13 pierde su persistencia real si se exporta a Web; documentado en SPEC.md.
- Sin presupuesto de tamaño (`wasm-opt`) ni botón "Exportar a Web" en el editor todavía — solo CLI, sin optimizar tamaño del binario.

---

### M15 — Exportador Android

Empaquetado elegido: **`cargo-apk`** (no `xbuild`) — es la herramienta que el propio ecosistema `winit`/`android-activity` da por hecha en sus ejemplos, y bastó con instalarla para producir un `.apk` real.

**Entregado:**
- `aigs_runtime::app`: `run_android(app: AndroidApp, …)` construye el `EventLoop` con `EventLoopBuilder::with_android_app` en vez de `EventLoop::new()`; el resto del bucle (`run_app`, bloqueante) es idéntico a Desktop — a diferencia de Web, Android sí puede bloquear su hilo con `pollster::block_on`, así que **no** necesitó la máquina de estados asíncrona del renderer de M14.
- Ciclo de vida: `suspended()` tira `window`/`renderer` (la superficie nativa muere al pasar a segundo plano); el `resumed()` ya existente los reconstruye sin cambios. Limitación conocida y documentada: las texturas no se recargan tras un resume (el `World` sobrevive, los sprites simplemente no se dibujan — sin crash, gracias a que `Renderer::render` ya indexaba texturas con `.get()`).
- `aigs_runtime::source::AndroidAssets`: lee del APK vía `AAssetManager`, **síncrono** (a diferencia de `MemoryAssets`/`fetch` en Web) — los assets ya están embebidos en el paquete, no hay red de por medio.
- Entrada táctil: `WindowEvent::Touch` alimenta el mismo `Input` que el ratón (posición + `click`), sin cambios de formato ni de behaviors existentes.
- Nuevo componente de formato `virtual_button: { key }` (ver SPEC.md): mientras se toca su sprite, simula esa tecla para behaviors/animators/scripts — necesario porque un juego de teclado (como Robot Rescue) no es jugable solo con tap-como-click.
- Nuevos crates: `exporters/android-player` (`aigs-android-player`, cdylib `aarch64-linux-android`, excluido del workspace nativo) es la **plantilla de build** del jugador — a diferencia de Web, Android no admite un artefacto único reutilizable porque empaqueta los assets dentro del APK en tiempo de compilación, así que cada export copia la plantilla, la retoca (assets del proyecto + `package`/`apk_name` únicos) y la compila de verdad. `exporters/android` (`aigs-export-android`, sí en el workspace) hace esa copia/retoque y ejecuta `cargo apk build` como subproceso.
- `aigs export --target android [--release]`: busca la plantilla en `<carpeta del ejecutable>/android-player-template/`; necesita NDK/SDK/`cargo-apk` instalados en la máquina que exporta (documentado, no es self-player al estilo M7).
- Verificado de punta a punta en este entorno: se instaló de verdad el NDK r28c + SDK (build-tools 34, platform 34) + `cargo-apk` 0.10.0, y se exportó Robot Rescue a un `.apk` firmado (keystore de depuración autogenerado) con todo el motor enlazado para `arm64-v8a`, confirmado con `aapt dump badging`.

**Pendiente / limitaciones conocidas:**
- **Validación manual en un dispositivo o emulador Android real todavía no hecha** (no hay ninguno disponible en este entorno de desarrollo) — falta confirmar que de verdad renderiza, acepta entrada táctil y suena al instalarlo y jugarlo.
- Sin `save.json` en Android todavía (mismo límite que Web).
- Build de depuración por defecto; release necesita que el usuario configure su propio keystore de firma.
- Sin botón "Exportar a Android" en el editor todavía (solo CLI).
- La plantilla distribuida junto a un CLI empaquetado (fuera de este monorepo) todavía depende de rutas relativas hacia `runtime/crates/`; empaquetar una plantilla verdaderamente autónoma para distribución amplia queda pendiente.

---

### M16 — Exportador iOS

**Tareas**
- Backend Metal de `wgpu`; generación de proyecto Xcode a partir del mismo pipeline de exportación.
- Reutilización del esquema de entrada táctil/botones virtuales de M15.
- Firma de código y perfiles de aprovisionamiento: documentado como paso manual del usuario (requiere macOS y cuenta de Apple Developer, fuera del control de CI).
- **Entregable:** proyecto Xcode generado que compila y corre en el simulador iOS; guía para llevarlo a un dispositivo físico.

---

### M17 — Optimización, paridad y publicación

**Tareas**
- Compresión de texturas y presupuesto de tamaño por plataforma; medición de arranque/FPS en gama media (no solo en la máquina de desarrollo).
- Matriz de CI que valida los cuatro exportadores (Desktop ya cubierto, + Web/Android/iOS nuevos) contra los ejemplos de `examples/`.
- **Menú de exportación en el editor:** el botón **⬇ Exportar** de la toolbar solo conoce Desktop (`export_project` en `editor/src-tauri/src/lib.rs` llama a `aigs export` sin `--target`, siempre con `--zip`). Pendiente: un desplegable Desktop/Web/Android(/iOS) que pase `--target` al CLI, sin `--zip` fuera de Desktop, y que en Android muestre con claridad si falta el NDK/SDK/`cargo-apk` en la máquina (error esperado, no un fallo críptico).
- Guía de usuario "publica tu primer juego" por tienda (Web/Play Store/App Store), con los pasos manuales de cada una explicados.
- **Entregable:** **release 0.3** — mismo proyecto exportado y verificado en las cuatro plataformas, exportable también desde el editor.

## Riesgos de la Fase 3

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Madurez de `wgpu` en backends móviles/web (WebGPU aún desigual entre navegadores) | Alto | *Fallback* a WebGL/GLES desde el diseño; M14 como piloto de bajo riesgo antes de móvil. |
| Modelo de entrada táctil vs. teclado/ratón rompe proyectos existentes | Alto | Botones virtuales como componente **opcional** del formato; behaviors/scripts no cambian su API. |
| Firma y publicación en tiendas depende de cuentas/credenciales del usuario, no automatizable | Medio | Documentar el proceso, no intentar gestionarlo desde CI; el exportador entrega el artefacto, no lo publica. |
| Tamaño de assets/binario inadecuado para móvil | Medio | Presupuesto de tamaño desde M14, compresión de texturas en M17. |

---

# Fase 4 — IA profunda

## Objetivo de la Fase 4

Pasar de "la IA puede leer y escribir el formato porque es JSON legible" (verdad desde el MVP) a **la IA como colaborador activo**: un chat nativo en el editor que entiende el proyecto abierto, propone y aplica cambios sobre el mismo `.aigs` que edita el usuario, y agentes especializados que cubren cada área del motor (ver [ia.md](ia.md) para la visión completa).

**Criterio de éxito:** describir un juego en una frase ("un juego de plataformas donde un robot rescata a su mascota") y obtener un proyecto `.aigs` jugable, editable después a mano en el editor como cualquier otro.

### Dentro del alcance

- AI Core: capa de contexto de proyecto + abstracción de proveedor de modelo (local y cloud).
- Chat con capacidad de **proponer y aplicar** cambios (no solo responder preguntas), siempre revisables/deshacibles.
- Agentes especializados por dominio del motor, coordinados sobre el mismo contexto.
- Generación de juegos completos de punta a punta a partir de lenguaje natural.

### Fuera del alcance (fase posterior)

- Agentes/modelos de terceros instalables por la comunidad → Fase 5 (SDK de plugins).

## Hitos de la Fase 4

### M18 — AI Core y chat con contexto

**Tareas**
- Capa de proveedores: **Ollama** (local, privado, sin coste) y **Claude/GPT/Gemini** (cloud) tras una interfaz común.
- Serialización del contexto relevante del proyecto abierto (escena activa, entidades, componentes, assets) al formato que consume el modelo, con límites de tamaño de contexto.
- Panel de **Chat** en el editor (evoluciona el *stretch goal* de M6): de solo lectura a empezar, responde preguntas correctas sobre el proyecto abierto.
- **Entregable:** preguntar "¿qué comportamientos tiene la entidad seleccionada?" y obtener una respuesta correcta basada en el `.aigs` real.

---

### M19 — Escritura asistida y primer agente (Programador)

**Tareas**
- El chat puede proponer cambios concretos al `.aigs` (crear entidad, añadir componente, generar un script `rhai`) como una propuesta con **diff/preview** que el usuario confirma antes de aplicar — mismo principio de "acción reversible, confirmación explícita" que gobierna este propio proyecto.
- Generación de scripts a partir de instrucciones en lenguaje natural, usando el manifiesto tipado de la API de scripting (`scripting-api.json`, ya existente desde M12) como contrato para que el modelo no invente funciones que no existen.
- **Entregable:** "añade un enemigo que patrulle entre estos dos puntos" genera la entidad, el sprite y el script funcionando, aplicado tras confirmación.

---

### M20 — Agentes especializados

**Tareas**
- Orquestación multi-agente sobre el contexto compartido del proyecto: Arquitecto (estructura/escenas), Animador (timelines/keyframes), Diseñador de niveles (composición), Audio, Física, Optimización (ver tabla completa en [ia.md](ia.md)).
- Cada agente limitado a su porción del formato, con el Arquitecto coordinando cambios que cruzan varias áreas.
- **Entregable:** una instrucción de alto nivel ("crea un segundo nivel más difícil, reusando los assets del primero") coordina varios agentes sin que el usuario tenga que orquestarlos a mano.

---

### M21 — Generación de juegos completos

**Tareas**
- Flujo de punta a punta descrito en [ia.md](ia.md): frase en lenguaje natural → proyecto `.aigs` jugable completo (escenas, personajes, física, animaciones, audio, código).
- Iteración conversacional para refinar el resultado ("hazlo más rápido", "que el enemigo tenga más vida").
- Caso de estudio documentado y reproducible: de prompt a juego jugable en minutos.
- **Entregable:** **release 0.4**, con el caso de estudio como demo pública del principio "la IA conoce el videojuego".

## Riesgos de la Fase 4

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Coste/latencia de modelos cloud en ediciones iterativas frecuentes | Alto | Proveedor local (Ollama) como opción por defecto; cloud opcional para tareas que lo justifiquen. |
| Cambios de agentes corrompen el proyecto o rompen el formato | Alto | Todo cambio de agente pasa por el mismo undo/redo del editor; preview/diff antes de aplicar, nunca escritura directa silenciosa. |
| Coordinación multi-agente inconsistente (dos agentes proponen cambios contradictorios) | Medio | Arquitecto como agente coordinador único con autoridad sobre conflictos; el resto no escribe directamente sin pasar por él. |
| El formato `.aigs` deja de ser la única fuente de verdad (agentes mantienen estado oculto) | Medio | Regla de arquitectura ya vigente desde el MVP: todo lo que el editor/IA puede hacer se expresa como datos en `.aigs`. |

---

# Fase 5 — Ecosistema

## Objetivo de la Fase 5

Que la comunidad pueda **extender AI Game Studio sin tocar el núcleo**: plugins de editor, componentes de runtime, exportadores y agentes IA de terceros, distribuidos a través de un marketplace, más colaboración en tiempo real sobre el mismo proyecto (ver [plugins.md](plugins.md) para la visión completa del SDK).

**Criterio de éxito:** un plugin de terceros (no escrito por el equipo del núcleo) instalado desde el marketplace añade un panel nuevo al editor o un componente nuevo al runtime, sin recompilar AI Game Studio.

### Dentro del alcance

- SDK público y estable para las extensiones ya previstas desde el MVP (paneles, componentes/sistemas, importadores, exportadores, agentes).
- Marketplace: publicación, descubrimiento, versionado e instalación desde el propio editor.
- Colaboración en tiempo real sobre un mismo proyecto.
- Servicios cloud **opcionales** (nunca obligatorios, coherente con la filosofía open-source/local-first del proyecto).

## Hitos de la Fase 5

### M22 — SDK de plugins v1

**Tareas**
- Estabilización de las APIs de extensión ya preparadas desde el MVP: paneles de editor, componentes/sistemas del runtime (catálogo, no hardcodeados en el loop), importadores de assets, exportadores de plataforma.
- Namespace propio para componentes de origen externo en el formato `.aigs` (ya soportado desde el diseño base — se documenta y estabiliza como contrato público).
- Plantilla de plugin de ejemplo + documentación de publicación en `sdk/`.
- **Entregable:** un plugin de ejemplo (panel de editor simple) instalable localmente sin tocar el core.

---

### M23 — Marketplace

**Tareas**
- Publicación, descubrimiento, versionado, instalación y actualización de plugins desde el editor.
- Cuenta de autor y moderación básica (revisión mínima antes de listar públicamente).
- **Entregable:** instalar un plugin de un tercero desde dentro del editor, sin pasos manuales de compilación.

---

### M24 — Colaboración en tiempo real

**Tareas**
- Edición concurrente del mismo proyecto por varias personas (mecanismo de resolución de conflictos sobre el `.aigs` estructurado — decisión de diseño registrada en arquitectura antes de implementar).
- Presencia de colaboradores en el editor (quién edita qué).
- Integración con control de versiones (posible flujo git nativo desde el editor, más allá de guardar archivos sueltos).
- **Entregable:** dos personas editando el mismo proyecto desde máquinas distintas sin pisarse los cambios.

---

### M25 — Servicios cloud opcionales

**Tareas**
- Guardado en la nube de proyectos y de partidas de jugador (evolución opcional de `save.json`, introducido en M13, hacia un servicio remoto para quien lo quiera — el modo local por archivo sigue siendo el por defecto).
- Analíticas básicas para juegos publicados (opt-in, nunca por defecto).
- **Entregable:** **release 1.0** — ecosistema completo: núcleo estable, SDK, marketplace, colaboración y servicios cloud opcionales.

## Riesgos de la Fase 5

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Plugins de terceros con código no confiable | Alto | Sandboxing/permutación de permisos por tipo de extensión; moderación mínima antes de listar en el marketplace. |
| Gobernanza del marketplace (calidad, spam, disputas) | Medio | Moderación básica desde M23; empezar con publicación manual antes de abrir auto-publicación. |
| Complejidad de resolución de conflictos en edición colaborativa sobre JSON estructurado | Alto | Decisión de mecanismo (CRDT u otro) registrada y prototipada antes de comprometerse en M24; alcance inicial puede limitarse a "un editor a la vez por escena" si el CRDT completo no está listo. |
| Servicios cloud contradicen la filosofía local-first/open-source del proyecto | Medio | Estrictamente opcionales y con alternativa local funcional; nunca requisito para usar el producto. |
