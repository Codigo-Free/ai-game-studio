# Arquitectura

AI Game Studio se compone de seis grandes bloques: **Editor, Runtime, Exporters, SDK, Plugins y AI Core**. Todos se comunican a través de un contrato común: el **formato de proyecto `.aigs`**.

---

## Principio rector

> Todo lo que el editor puede hacer se expresa como datos.

El proyecto completo (escenas, entidades, componentes, animaciones, assets, eventos) vive en archivos JSON legibles y versionados. Editor, runtime, exportadores e IA leen y escriben el mismo formato. Esto hace al sistema **AI-Ready**: un modelo de IA colabora manipulando datos, no simulando clics.

---

## Vista general

```
┌────────────────────────── Editor (Tauri) ──────────────────────────┐
│  React + TypeScript                                                │
│  Escena · Árbol · Inspector · Recursos · Timeline · Consola · IA   │
└───────────────▲────────────────────────────────▲───────────────────┘
                │ IPC (comandos/eventos)         │
┌───────────────▼───────────────┐   ┌────────────▼────────────┐
│        Runtime (Rust)         │   │   Proyecto .aigs (JSON) │
│  aigs-ecs · aigs-render(WGPU) │◄──│  escenas · entidades ·  │
│  aigs-anim · aigs-project     │   │  assets · animaciones   │
└───────────────▲───────────────┘   └────────────▲────────────┘
                │                                │
     ┌──────────┴──────────┐          ┌──────────┴──────────┐
     │      Exporters      │          │       AI Core       │
     │ Desktop·Android·Web │          │  Contexto + Agentes │
     └─────────────────────┘          └─────────────────────┘
```

---

## Módulos

### Editor
Aplicación de escritorio Tauri (Linux/Windows/macOS) con frontend React + TypeScript. Es la superficie visual del sistema: no contiene lógica de juego, opera sobre el modelo de documento y delega la ejecución al runtime embebido. Ver [editor.md](editor.md).

### Runtime
Motor de ejecución en Rust, organizado como workspace de crates:

| Crate | Responsabilidad |
|---|---|
| `aigs-ecs` | Entity Component System: entidades, componentes, sistemas, consultas. |
| `aigs-render` | Render 2D sobre WGPU: sprite batching, texturas, cámaras, capas. |
| `aigs-anim` | Evaluación de animaciones: pistas, keyframes, tweens, easing. |
| `aigs-project` | Carga/guardado y validación del formato `.aigs`; versionado y migraciones. |
| `aigs-runtime` | Capa de integración: componentes base, game loop, entrada, runner de ventana. |

Ver [runtime.md](runtime.md).

### Exporters
Convierten un proyecto `.aigs` en un binario distribuible por plataforma (Desktop en Fase 2; Android, Web e iOS en Fase 3). Ver [exportadores.md](exportadores.md).

### SDK
El contrato público del sistema: especificación del formato `.aigs`, APIs para plugins y bibliotecas cliente. Vive en `sdk/`.

### Plugins
Extensiones de comunidad sobre el SDK (Fase 5). Ver [plugins.md](plugins.md).

### AI Core
Capa que da a los modelos de IA (Ollama local; Claude, GPT y Gemini en cloud) acceso al contexto completo del proyecto y a acciones sobre el formato `.aigs`. Los agentes especializados se construyen sobre esta capa (Fase 4). Ver [ia.md](ia.md).

---

## Formato de proyecto `.aigs`

- **JSON** legible y autodescriptivo, con esquema y versión de formato explícita.
- Rutas de assets relativas a la raíz del proyecto.
- Un archivo de proyecto + un archivo por escena + catálogo de assets.
- Migraciones automáticas entre versiones del formato desde la Fase 1.
- SQLite se reserva para índices/caches locales del editor (miniaturas, búsqueda), nunca como fuente de verdad.

---

## Comunicación Editor ↔ Runtime

- El runtime corre embebido en el proceso Tauri.
- El frontend envía **comandos** (crear entidad, mover, play/stop) y recibe **eventos** (selección, logs, métricas) vía IPC de Tauri.
- El viewport de escena es renderizado por el runtime (WGPU); el prototipo técnico de esta integración es el primer paso del hito M3 del [plan](plan.md).

---

## Decisiones de arquitectura

Las decisiones relevantes se registran aquí antes de implementarse:

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-07 | Formato de proyecto JSON AI-Ready como contrato central | La IA y el editor deben operar sobre los mismos datos. |
| 2026-07 | ECS propio en lugar de bevy_ecs/hecs | Control total del diseño y del formato serializado; revisable si los benchmarks lo desaconsejan. |
| 2026-07 | WGPU como capa de render | Multiplataforma (Vulkan/Metal/DX12/WebGPU) y camino directo a Web en Fase 3. |
| 2026-07 | Tauri en lugar de Electron | Menor huella, backend Rust compartido con el runtime. |
| 2026-07 | Crate `aigs-runtime` como capa de integración (loop, componentes, runner) | Mantiene `aigs-ecs`/`aigs-render`/`aigs-anim` independientes y reutilizables. |
| 2026-07 | Game loop a paso fijo (60 Hz) con render interpolado vía `PrevTransform2D` | Simulación determinista independiente del refresco de pantalla. |
| 2026-07 | `wgpu` fijado en la serie 24 y `winit` en 0.30 durante el MVP | API estable conocida; la actualización a series nuevas se hará como tarea dedicada con benchmarks. |
| 2026-07 | En M1 el renderer dibuja a la ventana (winit); render offscreen para el viewport del editor se decide en M3 | Evita diseñar la integración editor↔runtime antes del prototipo de M3. |
| 2026-07 | **Viewport de edición en Canvas 2D (frontend), runtime real solo en Play** | El viewport de diseño dibuja el modelo de documento con la misma matemática TRS que el runtime; el botón Play guarda y lanza `aigs run` (WGPU real). Embeber WGPU bajo el webview de Tauri es frágil multiplataforma; se reevaluará para el modo Play integrado (M5). |
| 2026-07 | El modelo de documento vive en el frontend; el backend Tauri solo toca disco y procesos | Cada guardado se revalida con `aigs-project` (la implementación de referencia), garantizando que el editor nunca escribe un `.aigs` inválido. |
| 2026-07 | **Exportación desktop con diseño self-player**: el binario `aigs` ejecuta `data/game.aigs` si existe junto al ejecutable; exportar = copiar ejecutable + datos | Un solo binario (CLI + player + juego exportado), cero compilación para el usuario final, y el mismo artefacto ya probado en `aigs run`. Exportación cruzada se resolverá en Fase 3 con binarios por target. |
| 2026-07 | **Persistencia (M13): `save.json` fuera del formato `.aigs`, guarda solo la memoria de scripts (`get_var`/`set_var`) por id de entidad + marca de tiempo** | El estado de juego (partida) y los datos de diseño (proyecto) son cosas distintas; mezclarlos en `.aigs` rompería el principio de que el formato es contenido de diseño estático. Guardar solo la memoria de scripts (no todo el `World`) evita construir serialización genérica del ECS antes de que haya un caso de uso real que la necesite. |
| 2026-07 | **`elapsed_since_save()` se consume una sola vez por sesión (primera llamada de cualquier script; el resto reciben 0), no un valor que crece cada tick** | El tiempo offline se debe aplicar analíticamente una vez (p. ej. en `on_start`), no una y otra vez cada frame ni recalcular reproduciendo cada tick perdido — eso sería lento y frágil. Consumo único evita doble conteo aunque el script se lea desde varias entidades o tras cambiar de escena. |
| 2026-07 | La memoria de scripts (`get_var`/`set_var`) sobrevive a un cambio de escena dentro de la misma sesión, indexada por id de entidad autor | Antes se perdía al salir de la escena (bug de usabilidad para cualquier juego con más de una escena); ahora el guardado a disco y la persistencia entre escenas comparten la misma estructura interna en `ScriptHost`. |
| 2026-07 | Autoguardado periódico (sin hook de cierre limpio de ventana todavía) | Evita añadir una ruta de apagado especial en `aigs-runtime::run()` antes de que haya más de un caso de uso; pérdida máxima aceptada: la ventana entre autoguardados. |
| 2026-07 | `save.json` vive junto a los datos del proyecto, no en un directorio de perfil de usuario | Suficiente para el MVP de la demo; un directorio de guardado por usuario (XDG/AppData) queda para cuando haya más juegos que lo necesiten. |
| 2026-07-06 | **M14 (exportador Web): viabilidad de `wasm32-unknown-unknown` verificada por compilación real, no por suposición**, antes de comprometer el diseño de la Fase 3 | `aigs-render` (wgpu 24 + winit 0.30) compila para wasm32 sin cambios; `rapier2d`, `rhai`, `image` y `kira`/`cpal` también, una vez resuelto el bloqueo de `getrandom` (ver fila siguiente). Descarta el riesgo grande de la Fase 3 (¿corre el motor en el navegador?) al inicio de M14 en vez de al final. |
| 2026-07-06 | `getrandom` con feature `wasm_js` + `--cfg getrandom_backend="wasm_js"` como dependencia de target para `wasm32-unknown-unknown` | `getrandom` 0.3 (dependencia transitiva de `rapier2d`/`rhai`) no compila por defecto en wasm32-unknown-unknown (no hay fuente de aleatoriedad del sistema); es la solución oficial del propio crate, no un parche nuestro. |
| 2026-07-06 | Carga de assets/escenas/scripts abstraída detrás de un backend por plataforma (archivo local en Desktop, `fetch` asíncrono en Web) | `kira::StaticSoundData::from_file` (basado en `std::fs`) no existe en builds wasm — solo expone `from_cursor`/`from_media_source` (bytes en memoria). La misma abstracción de "bytes de un asset" sirve para imágenes, escenas `.aigs` y scripts `.rhai`, evitando tener dos pipelines de carga distintos. |
| 2026-07-06 | Nuevo crate `exporters/web` (misma interfaz que `exporters/desktop` de M7) para el exportador WASM | Mantiene el diseño ya establecido de "un exportador por plataforma" en `exporters/`; el pipeline común (validar → empaquetar → generar artefacto) se reutiliza, solo cambia el artefacto final (`index.html` + `.wasm` + assets en vez de un ejecutable). |
| 2026-07-06 | Hot reload de scripts por `mtime` (M12) se desactiva en builds wasm, sin afectar Desktop | El navegador no tiene acceso a un sistema de archivos observable del proyecto; no hay equivalente razonable a vigilar `mtime` de un `.rhai` en Web. |
| 2026-07-06 | `AssetSource` implementado para `PathBuf`, no para `Path` | `Path` es un tipo `?Sized` (como `str`); el compilador rechaza construir `&dyn AssetSource` a partir de `&Path` porque la coerción a trait object exige un tipo `Sized` en origen. `PathBuf` sí lo es y ya es el tipo que tiene `root` en todos los call sites reales (CLI), así que no cambia nada para quien lo usa. |
| 2026-07-06 | Inicialización del renderer como máquina de estados (`Uninitialized → Pending → Ready` vía `PendingRenderer`), no un `Renderer::new` bloqueante compartido entre plataformas | `Renderer::new` usa `pollster::block_on`, que en Web bloquearía el único hilo de JS del navegador esperando una `Promise` que nunca puede resolverse mientras ese hilo esté parado (deadlock real, no solo lento). En Web, `resumed()` lanza `Renderer::new_async` con `wasm_bindgen_futures::spawn_local` y un `Rc<RefCell<Option<Result<Renderer, RenderError>>>>` compartido; `tick()` comprueba si ya resolvió antes de intentar renderizar. Desktop no cambia de comportamiento (resuelve en el mismo tick, como siempre). |
| 2026-07-06 | Dos crates separados para Web: `exporters/web-player` (el jugador wasm, `cdylib`, excluido del workspace nativo) y `exporters/web` (`aigs-export-web`, empaquetador nativo, sí es miembro del workspace) | El jugador solo tiene sentido compilado a `wasm32-unknown-unknown` (un `cargo build --workspace` nativo no puede ni debe intentar producirlo, igual que `editor/src-tauri`); el empaquetador es código nativo normal que la CLI enlaza como cualquier otra dependencia. |
| 2026-07-06 | El jugador Web es un artefacto **prebuilt**, buscado por la CLI en `<carpeta del ejecutable>/web-player/` | A diferencia de Desktop (donde el propio binario `aigs` es el self-player), no existe un "yo mismo" en WASM: `aigs export --target web` sigue sin compilar nada (coherente con el diseño self-player de M7), pero necesita que ese jugador ya exista, compilado una vez con `wasm-bindgen` (ver CI). |
| 2026-07-06 | **M15 (exportador Android): viabilidad verificada instalando de verdad el NDK (r28c) + SDK (build-tools 34, platform 34) + `cargo-apk` 0.10.0** y produciendo un `.apk` firmado real con todo el motor enlazado (rapier2d, rhai, kira, wgpu/Vulkan), no solo `cargo check` | Mismo principio que M14: descartar el riesgo grande (¿enlaza y empaqueta de verdad nuestro grafo de dependencias completo para `aarch64-linux-android`?) al principio, con un artefacto real verificable (`aapt dump badging`), no una suposición. |
| 2026-07-06 | En Android, `AssetSource` lee de forma **síncrona** (`AndroidAssets` sobre `AAssetManager`), sin la máquina de estados asíncrona que necesitó Web | Los assets del proyecto se empaquetan dentro del propio APK en tiempo de build (no hay `fetch` de red); leerlos es tan síncrono como leer de disco en Desktop. Por la misma razón, `pollster::block_on` (usado por `Renderer::new`) funciona bien en Android — es un hilo de SO real, no el único hilo de JS de un navegador — así que el arranque del renderer tampoco necesita el `PendingRenderer` de Web. |
| 2026-07-06 | Ciclo de vida de Android (`suspended`/`resumed` de winit) tira `window`/`renderer` en `suspended()` y deja que el `resumed()` ya existente los reconstruya | Android destruye la superficie nativa al pasar la app a segundo plano; seguir usándola provocaría un crash. **Limitación conocida:** las texturas de la GPU no se recargan tras un resume (no se vuelve a ejecutar `setup`, que es donde se suben); `Renderer::render` indexa `TextureId` con `.get()` (nunca `[]`), así que el resultado es que los sprites no se dibujan tras una reanudación, no un crash — aceptable para esta primera versión, con la partida en curso preservada en el `World`. |
| 2026-07-06 | Entrada táctil (M15): un dedo alimenta el mismo modelo de `Input` que el ratón (`WindowEvent::Touch` → `set_mouse_position` + `on_mouse_button`), sin tocar el formato ni los behaviors existentes | Cualquier proyecto que ya use `click`/posición de ratón funciona en pantalla táctil sin cambios. Solo se sigue el primer dedo — multitáctil no es necesario para el alcance de M15. |
| 2026-07-06 | Nuevo componente `virtual_button` (`{ key }`) + `sync_virtual_buttons` en `GamePlayer`, reutilizando el mismo hit-test (rotación/escala) que ya usaba `click` | Un juego pensado para teclado (como Robot Rescue) no es jugable solo con tap-como-click; hacía falta un botón en pantalla que **sea** la tecla para behaviors/animators/scripts, sin inventar un evento nuevo que cada proyecto tendría que aprender. Se libera solo (consume-vs-no-consume por tick), nunca se queda "pegado". |
| 2026-07-06 | El exportador Android **no puede** ser self-player al estilo M7/M14: el jugador genérico (`exporters/android-player`) es una **plantilla de build**, no un artefacto único reutilizable | A diferencia de Web (que hace `fetch` del proyecto en tiempo de ejecución), Android empaqueta los assets dentro del APK en tiempo de compilación — no hay forma de "un binario genérico + datos externos" sin pedir permisos de almacenamiento amplios al usuario final, algo que se descartó deliberadamente. Por eso `aigs export --target android` sí necesita NDK/SDK/`cargo-apk` en la máquina que exporta (documentado como requisito, igual que Xcode para iOS en M16). |
| 2026-07-06 | Cada export Android reescribe `package`/`apk_name` en una copia de la plantilla (p. ej. `studio.aigamestudio.player.robot_rescue`) | Sin esto, exportar dos juegos distintos produciría el mismo id de paquete y no podrían instalarse a la vez en el mismo dispositivo. |
| 2026-07-06 | **M16 (iOS) diferido sin fecha**, decisión del usuario | Generar y compilar un proyecto Xcode real necesita macOS + Xcode, que no existen en este entorno de desarrollo — a diferencia de Android, no hay forma de instalar el toolchain que falta aquí. Se prioriza cerrar M17 (que no depende de iOS) y saltar a la Fase 4 antes que dejar la Fase 3 indefinidamente a medias; se retoma cuando haya una máquina macOS disponible. |
| 2026-07-06 | **M17: menú de exportación Desktop/Web/Android en el editor** en vez de mantenerlo solo-CLI | El backend Tauri (`export_project`) ya envolvía el mismo `aigs export` que usa el CLI; añadirle un parámetro `target` y reenviarlo fue mecánico porque el CLI ya validaba/reportaba errores con claridad (plantilla o toolchain faltante) — no hizo falta lógica nueva de manejo de errores en el editor. |
| 2026-07-06 | **M17: presupuesto de tamaño** vía `opt-level = "s"` + `lto = true` + `strip = true` en el perfil `release` de ambos jugadores, más `wasm-opt -Oz` sobre el `.wasm` en CI | Reduce el tamaño del binario sin tocar una sola línea del motor — la primera palanca de optimización a activar siempre, antes de plantearse algo más invasivo como compresión de texturas (diferido, sin máquina de gama media aquí para medir el efecto real). Medido en este entorno: el `.so` de Android en release (con todo el motor: rapier2d, rhai, kira, wgpu) queda en **6,8 MB** con estas tres flags. `wasm-opt` no se pudo probar localmente (el crate `wasm-opt` de Rust falló al compilar el binaryen vendorizado con el toolchain de esta máquina, un problema de esa vía de instalación, no del proyecto); en CI se instala el paquete `binaryen` del sistema (prebuilt, sin compilar C++) y ahí sí corre — el job *Web player* imprime el tamaño antes/después en su log: **3,33 MB → 2,94 MB** (~12 %) sobre el `.wasm` de Robot Rescue. **Gotcha real encontrado en CI:** `wasm-opt` por defecto valida contra el set de features "MVP" de WebAssembly y rechaza como inválido el `.wasm` que emite el target `wasm32-unknown-unknown` de rustc (usa `bulk-memory`/`sign-ext`, habilitadas por defecto en el toolchain estable) — hace falta `-all` (o los `--enable-*` concretos) para que wasm-opt acepte el mismo binario que el navegador ya ejecuta sin problema. |
| 2026-07-06 | **M18 (AI Core): proveedores como `enum`, no `dyn Trait`** | Con solo 2-3 proveedores (Ollama, Claude, y GPT/Gemini más adelante) y llamadas async, un trait object dinámico exigiría el crate `async-trait` (boxing de futures) solo para tener 2 variantes; un `enum Provider { Ollama(..), Claude(..) }` con `match` en un único `async fn chat` evita esa dependencia y es igual de fácil de extender. |
| 2026-07-06 | **M18: el contexto del proyecto para el chat lo construye el frontend, no el backend** | El frontend ya tiene en memoria el estado exacto que el usuario está editando (incluyendo cambios sin guardar); reconstruirlo leyendo de disco en el backend respondería con la última versión guardada, no con lo que se ve en pantalla — y hubiera duplicado el modelo de datos del documento en Rust sin necesidad. |
| 2026-07-06 | **M18: selección de proveedor/modelo por variables de entorno** (`AIGS_AI_PROVIDER`, `OLLAMA_MODEL`, `ANTHROPIC_API_KEY`), sin panel de configuración todavía | Evita construir una UI de ajustes (con guardado seguro de API keys) antes de saber si el propio chat es útil; documentado como límite conocido de M18, panel de ajustes previsto como fast-follow. |
| 2026-07-06 | **M18: verificado con Ollama real** (`deepseek-r1:7b`, `qwen2.5-coder:7b`, `llama3.2` ya estaban instalados y corriendo en este entorno) | Mismo principio que M14/M15: probar contra un servicio real en vez de asumir que el formato de petición/respuesta de la API es correcto. El proveedor Claude se implementó contra la documentación pública de la Messages API pero **no se pudo probar** sin una API key real (no se le pide una al usuario ni se expone en el chat por razones de seguridad) — queda marcado como no verificado hasta que alguien lo use con su propia key. |
| 2026-07-06 | **M19 (escritura asistida): el modo "proponer cambios" es un toggle explícito en el Chat, no una decisión del propio modelo** | Un modelo local pequeño (3-7B) decidiendo por sí mismo si debe responder texto o JSON estructurado es un punto de fallo extra e innecesario; el usuario ya sabe si quiere preguntar o pedir un cambio, así que el modo se fija en la petición (el system prompt cambia según el modo) en lugar de inferirse. |
| 2026-07-06 | **M19: el modelo propone un único objeto JSON (`ChangeProposal`: altas/bajas/parches de entidades + scripts nuevos) en vez de un bucle de *tool calling* multi-turno** | Ollama y Claude tienen protocolos de *function calling* distintos; mantener la misma abstracción `Provider::chat` de M18 para ambos (con `format:"json"` de Ollama como ayuda, no como requisito, y extracción tolerante del primer objeto `{...}` de la respuesta para Claude) evita duplicar la lógica de orquestación por proveedor a cambio de una propuesta "todo o nada" por turno — suficiente para el alcance de M19 (una instrucción → un cambio revisable), no para una conversación de agente con múltiples pasos de herramienta. |
| 2026-07-06 | **M19: la propuesta se valida en Rust contra los tipos reales del formato (`aigs_project::EntityNode`/`Components`), no contra un esquema JSON reinventado en TypeScript o en el prompt** | `EntityNode`/`Components` ya derivan `Deserialize`+`Serialize`; deserializar directamente la propuesta del modelo con esos tipos rechaza con un error claro cualquier componente inventado o mal tipado, sin mantener una segunda definición del formato solo para esto. Las referencias a assets (`sprite.asset`, `script.asset`, `particles.asset`) se comprueban a mano contra la lista de assets conocidos + los scripts que la propia propuesta declara, y el contenido de cada script nuevo se compila de verdad con `rhai::Engine::compile` antes de mostrarlo — así un script con un error de sintaxis nunca llega a la tarjeta de "Aplicar". |
| 2026-07-06 | **M19: aplicar una propuesta reutiliza `UPDATE_DOCUMENT`/undo del editor** (ya existente desde M3), sin un mecanismo de persistencia o histórico nuevo | El principio "acción reversible, confirmación explícita" que gobierna este propio proyecto (ver CLAUDE.md) aplica igual a cambios propuestos por la IA: aplicar es un commit de historial más, así que deshacer un cambio de la IA es exactamente `Ctrl+Z`, sin código especial. |
| 2026-07-06 | **M19: los scripts generados se escriben en `assets/` (mismo directorio que `import_asset`)**, no en un `scripts/` aparte como en los ejemplos más antiguos (p. ej. `robot-rescue/scripts/patrol.rhai`, escrito a mano antes de que el editor soportara importar `.rhai`) | El formato no exige ninguna estructura de carpetas (`path` es una ruta relativa libre); seguir la convención que ya usa el editor para *cualquier* asset importado (incluidos `.rhai` manuales) es más consistente que introducir una segunda convención solo para los scripts que escribe la IA. |
