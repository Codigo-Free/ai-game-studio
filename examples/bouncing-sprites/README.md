# Bouncing Sprites

Demo de M1: cientos de sprites rebotando a 60 FPS. No es un proyecto
`.aigs` — es un binario Rust que usa `aigs-runtime` directamente como
biblioteca, sin editor ni datos externos.

## Cómo ejecutarlo

```bash
cargo run -p bouncing-sprites --release
```

## Controles

Ninguno: es una demo de solo rendimiento, los sprites rebotan solos
contra los bordes de la ventana.

## Qué demuestra

- El renderer WGPU con batching instanciado y cámara ortográfica,
  usado directamente sin pasar por el formato `.aigs` ni el editor.
