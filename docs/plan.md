# Plan Maestro — AI Game Studio

Este documento define el plan de desarrollo del proyecto, con foco en el detalle completo de la **Fase 1 (MVP)**. La visión general del producto está en [proyecto.md](proyecto.md).

---

## Fases del proyecto

| Fase | Nombre | Alcance | Estado |
|---|---|---|---|
| **1** | MVP | Editor visual, Timeline, Escenas, Assets, Runtime básico | 🟢 **Completada** — [release 0.1.0](https://github.com/agilphp/ai-game-studio/releases/tag/v0.1.0) (estado por hito en [ROADMAP.md](../ROADMAP.md)) |
| **2** | Motor completo | Animaciones avanzadas, Física, Audio, Partículas, Exportación Desktop | 🔵 Siguiente |
| **3** | Multiplataforma | Android, Web, iOS, Optimización | ⚪ Pendiente |
| **4** | IA profunda | Integración profunda con IA, Agentes, Generación automática de videojuegos | ⚪ Pendiente |
| **5** | Ecosistema | Marketplace, Plugins, Servicios Cloud, Trabajo colaborativo | ⚪ Pendiente |

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

### M13 — Demo de plataformas y release 0.2
- Juego de plataformas completo (spritesheets + física + audio + partículas) en `examples/`.
- Benchmarks nuevos (física, partículas), migración de formato v0→v1 si hubo cambios de esquema, CHANGELOG y **release 0.2** con instaladores + demo exportada como artefacto.

## Riesgos de la Fase 2

| Riesgo | Impacto | Mitigación |
|---|---|---|
| La física introduce no-determinismo o rompe el paso fijo | Alto | rapier con timestep fijo (ya lo usamos); tests de regresión de simulación. |
| Crecimiento del formato sin control (v0 → v1) | Alto | Cada hito que toque el formato actualiza SPEC + migración en `aigs-project` en el mismo PR. |
| Audio multiplataforma (backends del SO) | Medio | `kira`/`cpal` abstraen; smoke test por SO en CI (sin dispositivo: modo dummy). |
| Scripting abre superficie de seguridad/estabilidad | Medio | `rhai` sandboxeado (sin IO), límites de operaciones por tick. |

---

## Fases posteriores (resumen)

- **Fase 3:** exportadores **Android, Web (WASM) e iOS**, optimización de tamaño y rendimiento.
- **Fase 4:** Chat IA nativo, agentes especializados (Arquitecto, Programador, Animador, Diseñador de niveles…), generación de juegos completos a partir de lenguaje natural sobre el formato `.aigs`.
- **Fase 5:** SDK público de plugins, marketplace, servicios cloud y colaboración en tiempo real.
