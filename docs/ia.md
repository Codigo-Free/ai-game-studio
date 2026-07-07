# Inteligencia Artificial

La IA es el pilar diferenciador de AI Game Studio. No es un chatbot acoplado al editor: es una entidad que **comprende completamente el proyecto** — escenas, personajes, componentes, animaciones, recursos, código, eventos, estados y arquitectura.

> **La IA conoce el videojuego y el videojuego conoce la IA.**

---

## Cómo se hace posible: el formato AI-Ready

La integración profunda llega en la **Fase 4**, pero se prepara desde el MVP: todo el proyecto vive en el formato `.aigs` (JSON legible, autodescriptivo y documentado). La IA colabora leyendo y escribiendo los mismos datos que el editor — no simula clics ni genera artefactos opacos.

Esto significa que la calidad futura de la IA depende directamente de la claridad del formato definido en la Fase 1 (ver [arquitectura.md](arquitectura.md)).

---

## Capacidades objetivo (Fase 4)

- Crear videojuegos completos a partir de lenguaje natural. — 🟢 M21: **estructura** completa (escenas, entidades, física, comportamientos, scripts), con assets ya importados por el usuario (ver límite honesto más abajo).
- Diseñar personajes, enemigos y niveles. — 🟢 M20/M21 (Arquitecto, Diseñador de niveles).
- Programar comportamientos. — 🟢 M19/M20 (Programador).
- Optimizar código y detectar errores. — 🟤 Diferido (sin datos de perfilado que lleguen al modelo todavía).
- Explicar código y generar documentación. — 🟢 M18 (chat de solo lectura).
- Crear pruebas automáticas y refactorizar proyectos. — ⚪ Sin implementar.

### Flujo de trabajo típico

El usuario escribe: *"Crea un juego de plataformas donde un robot debe rescatar a su mascota."*

La IA crea el proyecto, genera el personaje, diseña el escenario, construye enemigos, configura físicas, agrega animaciones, implementa colisiones, genera música, crea efectos, produce el código y la documentación. Después, el desarrollador ajusta cualquier elemento mediante edición visual.

**Límite honesto (M21):** "genera música/efectos" y "diseña el escenario" significan *conectar y componer con assets que el usuario ya importó* (imágenes, audio), no generar arte o audio nuevo — no hay ningún modelo de generación de imágenes/sonido integrado en este proyecto. Si la instrucción necesita un asset que no existe, ese paso falla la validación exactamente igual que en M19/M20, con el mismo mensaje claro; el usuario debe importar al menos arte de marcador de posición primero.

---

## Agentes especializados

Sobre el AI Core se construyen agentes que colaboran compartiendo el contexto completo del proyecto:

| Agente | Rol | Estado |
|---|---|---|
| Productor | Decide qué escenas hace falta crear o modificar para una instrucción de alto nivel. | 🟢 M21 |
| Arquitecto | Estructura del proyecto, escenas y componentes; coordina al resto. | 🟢 M20 |
| Diseñador de niveles | Composición de escenarios (entidades + colisión). | 🟢 M20 |
| Programador | Comportamientos, behaviors y scripts. | 🟢 M19/M20 |
| Especialista en física | Cuerpos, colisionadores y gravedad de la escena. | 🟢 M20 |
| Especialista en audio | Música de la escena y efectos de sonido. | 🟢 M20 |
| Animador | Conecta animaciones **ya existentes** vía el componente `animator`. | 🟡 M20 (sin autoría de keyframes — eso sigue siendo del Timeline, manual) |
| Diseñador UI | Interfaces y menús del juego. | ⚪ Sin implementar |
| Especialista en optimización | Rendimiento y tamaño. | 🟤 Diferido — no hay datos de perfilado (tamaño de assets, FPS reales) que lleguen al modelo todavía |

---

## Proveedores de modelos

| Tipo | Proveedor |
|---|---|
| Local | **Ollama** (privacidad, sin coste, offline) |
| Cloud | **Claude, GPT, Gemini** |

La capa AI Core abstrae el proveedor: los agentes funcionan igual con modelos locales o cloud.

### AI Core y panel de Chat (M18)

Vive en el backend Tauri (`editor/src-tauri/src/ai.rs`), no en el frontend: necesita hacer peticiones HTTP (a Ollama o a la nube) y ahí es donde ya vive el resto de IO del editor; además evita exponer API keys de proveedores cloud en el bundle del frontend.

- **`Provider`** es un `enum` (`Ollama { .. } | Claude { .. }`), no un `dyn Trait` — con dos o tres proveedores y llamadas async, un trait object habría necesitado el crate `async-trait` solo para tener el mismo despacho dinámico que ya da un `match`.
- **Selección de proveedor/modelo por variable de entorno** (`AIGS_AI_PROVIDER` = `ollama` (default) | `claude`, `OLLAMA_MODEL`, `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL`) — todavía sin panel de ajustes en el editor (fast-follow).
- **El contexto lo construye el frontend**, no el backend: `ChatPanel.tsx` serializa el proyecto/escena tal como están en memoria (incluyendo cambios sin guardar) y se lo pasa al comando Tauri `ai_chat` como texto; el backend nunca vuelve a leer el proyecto de disco para esto. Límite simple de tamaño (~12000 caracteres) para no desbordar la ventana de contexto de un modelo local pequeño.
- **Panel de Chat** en el editor (pestaña junto a Timeline/Consola): de momento solo lectura — responde preguntas sobre el proyecto abierto, no aplica cambios (eso es M19).
- Verificado con **Ollama real** corriendo en local (`llama3.2`, `qwen2.5-coder`, `deepseek-r1` ya instalados). El proveedor Claude se implementó contra la documentación pública de la Messages API pero no se ha podido probar sin una API key real de un usuario.

### Escritura asistida: primer agente "Programador" (M19)

El Chat gana un segundo modo, **"Proponer cambios"**, elegido explícitamente por el usuario (no algo que el modelo decide por sí mismo — más fiable con modelos locales pequeños). En ese modo, el modelo responde con un único objeto JSON (`ChangeProposal`: entidades a añadir/actualizar/eliminar + scripts `.rhai` nuevos) en vez de texto libre.

- **Validación real, no solo esperanza:** la propuesta se deserializa contra los tipos Rust reales del formato (`aigs_project::EntityNode`/`Components`) — un componente inventado o mal tipado se rechaza ahí mismo. Las referencias a assets (`sprite`/`script`/`particles`) se comprueban contra los assets del proyecto; cada script nuevo se **compila de verdad** con el motor rhai antes de mostrarse; los ids de entidad se comprueban contra los ya existentes en la escena. Todo o nada: o la propuesta entera pasa la validación, o se rechaza con un motivo claro — nunca una aplicación parcial.
- **El manifiesto de la API de scripting** (`scripting-api.json`, M12) se incluye en el prompt como contrato de funciones válidas, tal como preveía el plan.
- **Aplicar es un cambio más del historial de undo/redo del editor** (ya existente desde M3): deshacer una propuesta de la IA es `Ctrl+Z`, sin mecanismo especial.
- **Verificado de punta a punta con Ollama real** (`qwen2.5-coder:7b`): un pedido en lenguaje natural ("añade una moneda en esta posición usando este sprite") produjo una propuesta válida que pasó la validación sin cambios.

### Orquestación multi-agente (M20)

El Chat gana un tercer modo, **"Orquestar agentes"**: una instrucción de alto nivel se reparte automáticamente entre varios especialistas sin que el usuario tenga que invocarlos uno a uno.

- **Planificación en dos fases, determinista** (no un agente conversacional con bucle de herramientas): el **Arquitecto** responde un plan (`{"summary", "steps": [{"agent", "instruction"}]}`, máximo 8 pasos) con el mismo mecanismo de JSON validado de M19; cada paso se ejecuta **en secuencia** contra el *prompt* de su especialista.
- **Alcance de escritura por lista blanca de componentes**, comprobado en Rust (no solo pedido en el *prompt*): cada agente solo puede tocar los componentes que le corresponden — Arquitecto (`transform2d`/`sprite`), Diseñador de niveles (+ `collider2d`), Programador (`script`/`behaviors`), Física (`rigidbody2d`/`collider2d`/gravedad de escena), Audio (`behaviors`/música de escena), Animador (`animator`, solo animaciones ya existentes).
- **Un paso puede referenciar lo que creó el paso anterior**: los ids/assets/animaciones de cada paso se acumulan para el siguiente, dentro del mismo plan.
- **Todo o nada**: si cualquier paso falla la validación, se rechaza la orquestación completa, señalando qué paso y qué agente falló — nunca una aplicación parcial.
- **El resultado se fusiona en el mismo `ChangeProposal` de M19**: la tarjeta de propuesta y el aplicar/deshacer del editor no cambiaron nada para soportar esto.
- **Verificado de punta a punta con Ollama real** (`qwen2.5-coder:7b`, planificación + 2 pasos): una primera ejecución real expuso la validación funcionando exactamente como se diseñó — el paso de Diseñador de niveles propuso `"shape": "rectangle"` (valor que el formato no admite, solo `"box"`/`"circle"`) y la orquestación completa se rechazó con un mensaje señalando el paso y el motivo; se ajustó el *prompt* para deletrear los valores exactos de esos enums, y una segunda ejecución confirmó un plan de dos pasos (Arquitecto coloca la plataforma, Diseñador de niveles le añade un colisionador de caja) generado, validado y fusionado correctamente.

### Generación de juegos completos (M21)

El Chat gana un cuarto modo, **"Generar juego"**: una instrucción de alto nivel puede crear escenas nuevas enteras, no solo modificar la ya abierta — cerrando una brecha que había quedado en M20 (su propio ejemplo, "crea un segundo nivel", necesitaba una escena nueva que M20 nunca soportó por sí solo).

- **Un nuevo rol, "Productor"**, planifica por encima del Arquitecto: decide qué escenas hacen falta (la ya abierta y/o nuevas) y qué debe lograr cada una, y luego cada escena se construye reutilizando el **mismo** motor de M20 (Arquitecto → especialistas, alcance por lista blanca, todo o nada) sin ningún cambio.
- **Aplicar un juego generado es un único commit de historial**: todas las escenas nuevas/actualizadas se escriben a la vez, así que deshacerlo es un solo `Ctrl+Z`.
- **La iteración conversacional ("hazlo más rápido") no necesitó código nuevo**: el chat ya reconstruye su contexto desde el estado en memoria en cada mensaje, así que pedir un ajuste después de generar un juego es simplemente otra petición de "Proponer cambios"/"Orquestar agentes" sobre lo que ya existe.
- **Verificado de punta a punta con Ollama real** (`qwen2.5-coder:7b`): un caso de estudio real generando un mini-juego de dos escenas (menú con el sprite del héroe centrado + nivel con una plataforma estática usando el mismo sprite) a partir de una única instrucción. **Hicieron falta cinco intentos reales** (5-14 minutos cada uno, en CPU) para llegar a un resultado válido; los cuatro primeros expusieron, cada uno, un fallo de validación genuino y distinto (un campo de texto obligatorio en `null`, un script anidado como objeto en vez de referenciado por id, y dos veces un especialista intentando recrear con `entities_to_add` una entidad que otro paso del mismo plan ya había creado, en vez de usar `entities_to_update`) — la validación rechazó cada uno correctamente, señalando el paso y el motivo exactos, nunca aplicando nada parcial; cada hallazgo se corrigió aclarando el *prompt* compartido de todos los agentes (beneficia también a M19/M20). El quinto intento generó, validó y fusionó las dos escenas correctamente, con una jerarquía padre-hijo razonable en el menú y la plataforma del nivel construida con altas y actualizaciones coherentes sobre la misma entidad.

---

## Hoja de ruta de la IA

| Fase | Alcance IA |
|---|---|
| 1 (MVP) | Formato `.aigs` AI-Ready — el contrato que hace posible todo lo de más abajo. |
| 4 | Chat IA nativo: de solo lectura (M18) a escritura asistida con propuesta/confirmación (M19), agentes especializados (M20) y generación de juegos completos (M21) — **fase cerrada**, los cuatro hitos verificados con Ollama real. |
| 5 | Agentes de comunidad vía SDK de plugins. |
