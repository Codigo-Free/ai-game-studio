# Inteligencia Artificial

La IA es el pilar diferenciador de AI Game Studio. No es un chatbot acoplado al editor: es una entidad que **comprende completamente el proyecto** — escenas, personajes, componentes, animaciones, recursos, código, eventos, estados y arquitectura.

> **La IA conoce el videojuego y el videojuego conoce la IA.**

---

## Cómo se hace posible: el formato AI-Ready

La integración profunda llega en la **Fase 4**, pero se prepara desde el MVP: todo el proyecto vive en el formato `.aigs` (JSON legible, autodescriptivo y documentado). La IA colabora leyendo y escribiendo los mismos datos que el editor — no simula clics ni genera artefactos opacos.

Esto significa que la calidad futura de la IA depende directamente de la claridad del formato definido en la Fase 1 (ver [arquitectura.md](arquitectura.md)).

---

## Capacidades objetivo (Fase 4)

- Crear videojuegos completos a partir de lenguaje natural.
- Diseñar personajes, enemigos y niveles.
- Programar comportamientos.
- Optimizar código y detectar errores.
- Explicar código y generar documentación.
- Crear pruebas automáticas y refactorizar proyectos.

### Flujo de trabajo típico

El usuario escribe: *"Crea un juego de plataformas donde un robot debe rescatar a su mascota."*

La IA crea el proyecto, genera el personaje, diseña el escenario, construye enemigos, configura físicas, agrega animaciones, implementa colisiones, genera música, crea efectos, produce el código y la documentación. Después, el desarrollador ajusta cualquier elemento mediante edición visual.

---

## Agentes especializados

Sobre el AI Core se construirán agentes que colaboran compartiendo el contexto completo del proyecto:

| Agente | Rol |
|---|---|
| Arquitecto | Estructura del proyecto, escenas y componentes. |
| Diseñador UI | Interfaces y menús del juego. |
| Programador | Comportamientos y lógica. |
| Animador | Timelines, keyframes y transiciones. |
| Diseñador de niveles | Composición de escenarios. |
| Especialista en física | Colisiones y dinámicas. |
| Especialista en audio | Música y efectos de sonido. |
| Especialista en optimización | Rendimiento y tamaño. |

---

## Proveedores de modelos

| Tipo | Proveedor |
|---|---|
| Local | **Ollama** (privacidad, sin coste, offline) |
| Cloud | **Claude, GPT, Gemini** |

La capa AI Core abstrae el proveedor: los agentes funcionan igual con modelos locales o cloud.

---

## Hoja de ruta de la IA

| Fase | Alcance IA |
|---|---|
| 1 (MVP) | Formato `.aigs` AI-Ready. *Stretch:* panel de chat experimental (Ollama) con lectura del contexto del proyecto. |
| 4 | Chat IA nativo con escritura sobre el proyecto, agentes especializados, generación de juegos completos. |
| 5 | Agentes de comunidad vía SDK de plugins. |
