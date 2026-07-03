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
- Benchmarks de ECS (iteración masiva de entidades) y de render (sprite batching) con Criterion, desde el hito M1.
- Presupuesto de frame: 60 FPS estables en la demo del MVP; regresiones vigiladas en CI.

---

## Herramientas

| Área | Herramienta |
|---|---|
| Rust | `cargo test`, Criterion (benchmarks), clippy + rustfmt |
| TypeScript | Vitest, React Testing Library, eslint + prettier |
| E2E editor (post-MVP) | WebDriver de Tauri |

Los tests corren en cada push mediante GitHub Actions ([ci-cd.md](ci-cd.md)).
