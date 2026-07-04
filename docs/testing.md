# Testing

La calidad es transversal a todos los hitos del [plan](plan.md): **ningún hito se cierra sin tests de su alcance**.

---

## Niveles de prueba

### Unitarios
- **Runtime (Rust):** ECS (entidades, consultas, sistemas), evaluación de animaciones (keyframes, tweens, easing), serialización del formato `.aigs` (ida y vuelta sin pérdida), migraciones de versión de formato.
- **Editor (TypeScript):** modelo de documento, undo/redo, reducers/estado de paneles (Vitest + React Testing Library).

### Integración
- Cargar un proyecto `.aigs` → ejecutar la escena en el runtime → verificar estado resultante.
- Comandos IPC editor ↔ runtime: cada comando produce el evento/estado esperado.
- Proyectos de ejemplo de `examples/` como fixtures: deben cargar y ejecutar en cada build.

### Rendimiento
- Benchmarks con Criterion en `runtime/crates/aigs-ecs/benches` y `aigs-anim/benches`; ejecutar con `cargo bench -p aigs-ecs -p aigs-anim`. CI los compila (`--no-run`) para evitar que se pudran; las cifras se miden localmente.
- Presupuesto de frame: 60 FPS estables (~16,6 ms por frame).
- **Baseline 0.1.0** (Linux, 2026-07-03):

| Benchmark | Tiempo | Nota |
|---|---|---|
| `spawn_insert_10k` | ~458 µs | crear 10k entidades con 1–2 componentes |
| `query2_10k` | ~21,5 µs | iterar 5k entidades (Position+Velocity) — <0,2 % del frame |
| `sample_8kf` | ~8,7 ns | muestreo de pista típica |
| `sample_256kf` | ~115 ns | pista extrema (búsqueda lineal, optimizable a binaria si crece) |

- Benchmark de render (sprite batching) pendiente: requiere GPU; se medirá manualmente con `bouncing-sprites` (`AIGS_SPRITES=5000`).

---

## Herramientas

| Área | Herramienta |
|---|---|
| Rust | `cargo test`, Criterion (benchmarks), clippy + rustfmt |
| TypeScript | Vitest, React Testing Library, eslint + prettier |
| E2E editor (post-MVP) | WebDriver de Tauri |

Los tests corren en cada push mediante GitHub Actions ([ci-cd.md](ci-cd.md)).
