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
