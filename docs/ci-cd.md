# CI/CD

Automatización con **GitHub Actions** desde el hito M0 del [plan](plan.md).

---

## Pipeline de integración continua (cada push / PR)

1. **Lint y formato:** clippy + rustfmt (Rust), eslint + prettier (TypeScript).
2. **Build multiplataforma:** Linux, Windows y macOS (matrix).
3. **Tests:** unitarios e integración ([testing.md](testing.md)).
4. **Benchmarks:** comparación contra la línea base; una regresión significativa marca el build.
5. **Exportadores Web/Android** (M14/M15): jobs dedicados que instalan de verdad el toolchain de cada plataforma (`wasm-bindgen`, o NDK+SDK+`cargo-apk`) y ejecutan `aigs export --target web|android` sobre los ejemplos, no solo compilan — Web prueba los cuatro ejemplos (barato, sin recompilar nada por export); Android prueba Robot Rescue (cada export ahí recompila el motor entero, así que se limita al ejemplo que ya ejercita todos los tipos de asset).

## Quality gates

- Un PR no se fusiona con lint, build o tests en rojo.
- Los proyectos de `examples/` deben cargar y ejecutar en cada build.
- Cambios en el formato `.aigs` exigen actualizar la especificación en `sdk/` y las migraciones.

## Releases

- Versionado **SemVer**; `0.1.0` corresponde al cierre del MVP (hito M6).
- Tag → workflow de release: compila el editor para las tres plataformas de escritorio, genera instaladores y publica en **GitHub Releases** con changelog.
- Changelog generado a partir de commits convencionales (Conventional Commits), asistido por IA.

## Metodología AI First

La IA participa en el propio pipeline: revisión de código en PRs, generación de changelog, detección de documentación desactualizada respecto al código.
