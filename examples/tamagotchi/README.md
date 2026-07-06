# Tamagotchi

Demo de M13: una mascota virtual persistente. Todo el juego es un único
script (`scripts/pet.rhai`), sin código fuera de `.aigs`/`.rhai`.

## Cómo ejecutarlo

```bash
aigs run examples/tamagotchi/game.aigs
```

(o desde el editor: **Abrir…** → `examples/tamagotchi/game.aigs` → ▶ Play)

## Controles

Con la ventana del juego enfocada (haz clic sobre ella primero):

| Tecla | Acción |
|---|---|
| `1` | Alimentar (sube hambre) |
| `2` | Jugar (sube felicidad) |
| `3` | Medicina (sube salud) |

## Cómo funciona

- **Hambre** y **felicidad** decaen solas con el tiempo real; si alguna
  llega a 0, la **salud** también empieza a bajar (y sube si ambas están
  cubiertas).
- La cara de la mascota cambia sola según su estado: parpadeo normal,
  cara de comer, cara feliz, cara triste (hambre o felicidad por debajo
  de 30) o desmayada (salud a 0).
- Con la edad la mascota **crece** (más grande) si se la cuida bien.
- **Persiste entre partidas de verdad**: aunque cierres la ventana, la
  mascota sigue "viviendo". `aigs run` autoguarda cada ~10 s en
  `save.json` (no se sube a git — es estado de partida, no dato de
  diseño). Al reabrir, la consola indica cuánto tiempo pasó
  (`save: loaded (...s since last save)`) y las stats reflejan ese hueco.
- No hay marcador numérico en pantalla a propósito (fiel al juguete
  original): la cara y el tamaño son la única señal de estado.

## Limitación conocida

No hay mecánica de "limpieza" (no existe spawn dinámico de entidades
desde scripts todavía); si la salud llega a 0 la mascota se muestra
desmayada pero se recupera si vuelve a recibir cuidado.
