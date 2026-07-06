# Hello World

Proyecto mínimo válido: un menú y un nivel jugable, definidos por
completo como datos `.aigs` (sin física, audio ni scripting). Es el
fixture de referencia del CI.

## Cómo ejecutarlo

```bash
aigs run examples/hello-world/game.aigs
```

(o desde el editor: **Abrir…** → `examples/hello-world/game.aigs` → ▶ Play)

## Controles

| Pantalla | Entrada | Acción |
|---|---|---|
| Menú | clic o `Enter` | Empezar (va al nivel) |
| Nivel | flechas | Mover al héroe |
| Nivel | `Escape` | Volver al menú |
| Nivel | clic en la meta | Reproduce una animación |

## Qué demuestra

- Escenas, animaciones y comportamientos básicos (evento → acción) sin
  ningún sistema avanzado del motor.
