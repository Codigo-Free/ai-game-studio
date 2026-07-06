# Physics Playground

Demo de M8: cajas cayendo, una pelota que rebota, un robot cinemático que
empuja cajas y una estrella sensor que reacciona al contacto. Sin código,
solo física (`rigidbody2d`/`collider2d`) declarada en `.aigs`.

## Cómo ejecutarlo

```bash
aigs run examples/physics-playground/game.aigs
```

(o desde el editor: **Abrir…** → `examples/physics-playground/game.aigs` → ▶ Play)

## Controles

| Tecla | Acción |
|---|---|
| Flechas | Mover al robot (empuja las cajas que toca) |

No hay más interacción: las cajas y la pelota caen y rebotan solas por
gravedad/física, y la estrella emite partículas + sonido al detectar
contacto (sensor, no bloquea).

## Qué demuestra

- Cuerpos dinámicos (cajas, pelota) vs. cinemáticos (robot).
- Evento `collision` con filtro (`with`) disparando sonido/partículas.
- 60 FPS sostenidos con 10 entidades físicas activas.
