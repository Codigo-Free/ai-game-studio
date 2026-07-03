# Exportadores

Los exportadores convierten un proyecto `.aigs` en un producto distribuible por plataforma. Empaquetan el **runtime** junto con los datos y assets del proyecto.

El MVP (Fase 1) **no incluye exportación**: los juegos se ejecutan en el modo Play del editor. La exportación comienza en la Fase 2.

---

## Plataformas objetivo

| Plataforma | Fase | Salida |
|---|---|---|
| **Desktop** (Linux, Windows, macOS) | 2 | Binario nativo + assets empaquetados. |
| **Android** | 3 | APK/AAB. |
| **Web** | 3 | WASM + WebGPU/WebGL (el runtime ya usa WGPU, ver [runtime.md](runtime.md)). |
| **iOS** | 3 | Proyecto Xcode / IPA. |

---

## Diseño

- Cada exportador vive en `exporters/` como módulo independiente sobre una interfaz común (`export(proyecto, plataforma, perfil) → artefacto`).
- El pipeline: validar proyecto → compilar runtime para el target → empaquetar assets optimizados → generar artefacto.
- Perfiles de exportación (debug/release) definidos en el proyecto `.aigs`.
- En la Fase 5, la comunidad podrá aportar exportadores nuevos vía SDK ([plugins.md](plugins.md)).
