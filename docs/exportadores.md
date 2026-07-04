# Exportadores

Los exportadores convierten un proyecto `.aigs` en un producto distribuible por plataforma. Empaquetan el **runtime** junto con los datos y assets del proyecto.

---

## Plataformas

| Plataforma | Fase | Salida | Estado |
|---|---|---|---|
| **Desktop** (Linux, Windows, macOS) | 2 | Carpeta auto-contenida + `.zip` opcional. | 🟢 Disponible (M7) |
| **Android** | 3 | APK/AAB. | ⚪ |
| **Web** | 3 | WASM + WebGPU/WebGL (el runtime ya usa WGPU, ver [runtime.md](runtime.md)). | ⚪ |
| **iOS** | 3 | Proyecto Xcode / IPA. | ⚪ |

---

## Exportador Desktop (M7)

```bash
aigs export mi-juego/game.aigs --output dist --zip
```

Produce una carpeta lista para distribuir:

```
dist/mi-juego/
├── mi-juego[.exe]     # ejecutable del juego
└── data/
    ├── game.aigs      # manifiesto
    ├── scenes/…       # escenas del manifiesto
    └── assets/…       # assets del manifiesto
```

**Diseño self-player** (decisión en [arquitectura.md](arquitectura.md)): el binario `aigs` detecta al arrancar si existe `data/game.aigs` junto al ejecutable; si es así, ejecuta el juego directamente. Exportar consiste en copiar el ejecutable renombrado más los datos validados — sin compilación, sin toolchain para el usuario final.

- El editor expone el botón **⬇ Exportar** (guarda, elige carpeta y ejecuta `aigs export`).
- El exportador vive en `exporters/desktop` (`aigs-export-desktop`) y valida el proyecto completo antes de escribir nada; se niega a sobrescribir una exportación existente.
- Limitación M7: exporta para el **sistema operativo actual** (el binario empaquetado es el local). La exportación cruzada llega con los targets de Fase 3.

## Diseño general

- Cada exportador vive en `exporters/` como módulo independiente; el pipeline común es: validar proyecto → obtener runtime del target → empaquetar datos → generar artefacto.
- En la Fase 5, la comunidad podrá aportar exportadores nuevos vía SDK ([plugins.md](plugins.md)).
