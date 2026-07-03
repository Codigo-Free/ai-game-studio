# Editor Visual

El editor es la principal herramienta de desarrollo de AI Game Studio (principio **Visual First**). Es una aplicación de escritorio **Tauri + React + TypeScript** para Linux, Windows y macOS.

El editor no contiene lógica de juego: opera sobre el modelo de documento (formato `.aigs`) y delega render y ejecución al runtime embebido.

---

## Paneles

### MVP (Fase 1)

| Panel | Función |
|---|---|
| **Escena (viewport)** | Lienzo principal. Render vía runtime WGPU. Selección, movimiento, zoom, pan, gizmos de transformación. |
| **Árbol de objetos** | Jerarquía de entidades de la escena: crear, renombrar, eliminar, reordenar, reparentar. |
| **Inspector** | Propiedades de los componentes de la entidad seleccionada, editables en vivo. |
| **Recursos** | Catálogo de assets del proyecto. Importación por arrastre, miniaturas, arrastrar sprite → escena. |
| **Timeline** | Línea de tiempo estilo Flash: capas por entidad, fotogramas, keyframes, tweens, scrubbing, reproducción. |
| **Consola** | Logs del editor y del runtime, métricas (FPS, entidades, tiempo de frame). |

### Fases posteriores

| Panel | Fase |
|---|---|
| Animador avanzado (curvas, máquinas de estados) | 2 |
| Editor de físicas | 2 |
| Editor de partículas | 2 |
| **Chat IA** (asistente con contexto completo del proyecto) | 4 (prototipo de solo lectura como stretch del MVP) |

---

## Interacciones clave del MVP

- **Proyectos:** crear, abrir, guardar; archivos `.aigs` en disco, aptos para Git.
- **Composición:** arrastrar un sprite desde Recursos al viewport crea una entidad con `Transform2D + Sprite`.
- **Animación:** insertar keyframes sobre posición, rotación, escala, opacidad y frame de sprite; tween lineal y easing básico entre keyframes.
- **Escenas:** crear y navegar múltiples escenas; definir la escena inicial.
- **Modo Play:** ejecutar/pausar/detener el juego dentro del viewport sin corromper el estado de edición.
- **Undo/redo** global sobre el documento.

---

## Arquitectura interna

- **Modelo de documento** en el frontend: representación en memoria del `.aigs` con historial de cambios (undo/redo).
- Cada mutación del documento se refleja en el runtime vía comandos IPC; el runtime emite eventos (selección desde viewport, logs, métricas).
- Los paneles son módulos independientes sobre un sistema de docking, pensados para que futuros plugins (Fase 5) aporten paneles propios.
