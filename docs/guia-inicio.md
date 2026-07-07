# Guía de inicio rápido

Crea tu primer juego con AI Game Studio en unos minutos. Sin escribir código.

---

## 1. Requisitos

| Herramienta | Versión | Nota |
|---|---|---|
| Rust | estable (rustup) | compila runtime, CLI y backend del editor |
| Node.js | 20+ | frontend del editor |
| Linux | `webkit2gtk-4.1` | `sudo pacman -S webkit2gtk-4.1` (Arch) / `sudo apt install libwebkit2gtk-4.1-dev` (Debian/Ubuntu) |

> También puedes descargar instaladores del editor y binarios del CLI desde [GitHub Releases](https://github.com/Codigo-Free/ai-game-studio/releases).

## 2. Instalar y arrancar

```bash
git clone https://github.com/Codigo-Free/ai-game-studio
cd ai-game-studio

# CLI (lo usa el botón ▶ Play del editor)
cargo install --path cli

# Editor
cd editor && npm install && npm run tauri dev
```

## 3. Prueba la demo

1. En la pantalla de bienvenida pulsa **Abrir…** y elige `examples/robot-rescue/game.aigs`.
2. Pulsa **▶ Play**: haz clic en el robot para empezar, muévete con las **flechas**, haz clic en la mascota para rescatarla, y vuelve al menú con **Enter**. La pestaña **Consola** muestra los FPS y los cambios de escena en vivo.

También puedes jugarla sin el editor:

```bash
aigs run examples/robot-rescue/game.aigs
```

## 4. Tour del editor

```
┌──────────────────────── Toolbar ────────────────────────┐
│ Nuevo · Abrir · Guardar · ⤺⤻ · [escena ▾] ＋⧉★✕ · ▶ Play │
├─────────┬──────────────────────────────┬────────────────┤
│ Escena  │                              │   Inspector    │
│ (árbol) │        Viewport              │  Transform2D   │
├─────────┤   zoom: rueda · pan: Shift   │  Sprite        │
│Recursos │   arrastra para mover        │  Comportamien. │
├─────────┴──────────────────────────────┴────────────────┤
│ [Timeline | Consola]  pistas · keyframes · ▶ scrubbing   │
└──────────────────────────────────────────────────────────┘
```

- **Árbol de escena**: doble clic renombra; con la entidad seleccionada aparecen ▲▼ (orden), ＋ (hijo) y ✕.
- **Recursos**: importa imágenes PNG/JPG y **arrástralas al viewport** para crear sprites.
- **Inspector**: edita posición, rotación, escala, opacidad y capa; añade **comportamientos** ("Cuando tecla mantenida → mover").
- **Timeline**: crea una animación (＋), selecciona una entidad, añade una pista (＋ pista…), inserta keyframes con **doble clic**, arrástralos, y dale a ▶.
- **Atajos**: Ctrl+S guardar · Ctrl+Z/Ctrl+Shift+Z deshacer/rehacer · Supr eliminar entidad.

## 5. Tu primer juego en 8 pasos

1. **Nuevo** → elige una carpeta vacía y un nombre.
2. Importa un sprite en **Recursos** y arrástralo al lienzo.
3. En el **Inspector**, añade un comportamiento: *Cuando tecla mantenida `ArrowRight` hacer mover (300, 0)*. Repite para las otras flechas.
4. En el **Timeline**, crea una animación `idle` en loop y anima `transform2d.scale_x` con dos keyframes.
5. Crea una segunda escena con ＋ junto al selector de escenas (por ejemplo `nivel-2`).
6. Añade a algún sprite el comportamiento *Cuando clic hacer ir a escena `nivel-2`*.
7. **Guardar** (Ctrl+S) y **▶ Play**.
8. Tu juego es una carpeta con archivos `.aigs` legibles: versiónala con Git, edítala a mano o — pronto — deja que la IA la modifique contigo.

## 6. El CLI

```bash
aigs validate mi-juego/game.aigs   # valida manifiesto, escenas y assets
aigs run mi-juego/game.aigs        # ejecuta el juego (AIGS_MAX_FRAMES=N para smoke tests)
```

## ¿Y ahora?

- Formato `.aigs` a fondo: [sdk/aigs-format/SPEC.md](../sdk/aigs-format/SPEC.md)
- Qué viene después (física, audio, exportación, IA): [ROADMAP](../ROADMAP.md)
- Contribuir: [CONTRIBUTING.md](../CONTRIBUTING.md)
