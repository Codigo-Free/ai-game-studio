# Robot Rescue

Demo completo del MVP: un robot debe rescatar a su mascota. Menú → nivel
jugable → pantalla de victoria, encadenados con `animation_end` — todo
`.aigs` (comportamientos, animaciones, escenas) más un script corto
(`scripts/patrol.rhai`) para el dron patrullero.

## Cómo ejecutarlo

```bash
aigs run examples/robot-rescue/game.aigs
```

(o desde el editor: **Abrir…** → `examples/robot-rescue/game.aigs` → ▶ Play)

## Controles

| Pantalla | Entrada | Acción |
|---|---|---|
| Menú | clic o `Enter` | Empezar (va al nivel) |
| Nivel | flechas | Mover al robot |
| Nivel | `Escape` | Volver al menú |
| Nivel | clic en la mascota (meta) | Reproduce animación de rescate → victoria |
| Victoria | clic o `Enter` | Volver al menú |

## Qué demuestra

- Spritesheet + animator (el robot camina con 6 frames, idle↔walk según
  las flechas).
- Dron patrullero controlado por script (`patrol.rhai`), con memoria
  persistente vía `get_var`/`set_var`.
- Audio: tema de menú, "pop" al moverse, jingle de victoria.
- Multi-escena con `goto_scene` encadenado por evento (`animation_end`).
