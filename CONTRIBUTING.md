# Contribuir a AI Game Studio

¡Gracias por tu interés en contribuir! AI Game Studio es un proyecto open source y toda contribución es bienvenida: código, documentación, ejemplos, reporte de errores e ideas.

## Antes de empezar

1. Lee [docs/proyecto.md](docs/proyecto.md) para entender la visión y filosofía.
2. Revisa [docs/plan.md](docs/plan.md) para conocer el hito actual y su alcance.
3. Consulta [docs/arquitectura.md](docs/arquitectura.md) antes de proponer cambios estructurales.

## Flujo de trabajo

1. Abre un **issue** describiendo el bug o la propuesta antes de escribir código.
2. Haz fork y crea una rama desde `main`: `feat/<descripcion>` o `fix/<descripcion>`.
3. Desarrolla siguiendo las convenciones de abajo.
4. Abre un **pull request** hacia `main` enlazando el issue.

## Convenciones

### Commits
Usamos [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(runtime): añade sprite batching por textura
fix(editor): corrige undo en el inspector
docs(plan): actualiza hito M2
```

Tipos: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`, `ci`.

### Código
- **Rust:** `cargo fmt` + `cargo clippy` sin warnings; tests con `cargo test`.
- **TypeScript:** `prettier` + `eslint`; tests con `vitest`.
- Todo cambio de comportamiento incluye tests ([docs/testing.md](docs/testing.md)).
- Cambios en el formato `.aigs` exigen actualizar la especificación en `sdk/` y las migraciones.

### Documentación
- Las decisiones de arquitectura se registran en [docs/arquitectura.md](docs/arquitectura.md) antes de implementarse.
- Si tu cambio afecta un documento de `docs/`, actualízalo en el mismo PR.

## Quality gates

Ningún PR se fusiona con lint, build o tests en rojo. El pipeline de CI corre en cada push ([docs/ci-cd.md](docs/ci-cd.md)).

## Desarrollo AI First

Este proyecto se desarrolla con metodología AI First: se fomenta el uso de asistentes de IA (Claude, GPT, Ollama, Gemini) para diseñar, implementar, probar y documentar. Eres responsable de revisar y entender todo el código que envíes, sea generado por IA o no.

## Licencia

Al contribuir aceptas que tu aportación se licencia bajo [MIT](LICENSE).
