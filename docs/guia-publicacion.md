# Guía de publicación — publica tu primer juego (M17)

`aigs export` produce el artefacto (carpeta web, `.apk`…); **publicarlo en una tienda es un paso manual del usuario**, con sus propias cuentas y credenciales — nada de esto lo automatiza el proyecto ni corre en CI. Esta guía cubre los pasos concretos por plataforma, con lo que ya está disponible hoy (Desktop, Web, Android). iOS queda diferido (ver [plan.md](plan.md), M16) hasta que haya una máquina macOS para implementarlo.

---

## Web (cualquier hosting estático, o GitHub Pages)

```bash
aigs export mi-juego/game.aigs --target web --output dist
```

Esto produce `dist/mi-juego/` con `index.html` + el jugador + los datos del proyecto (ver [exportadores.md](exportadores.md)).

1. **Probarlo localmente primero:** `npx serve dist/mi-juego` y abre la URL que imprime. No sirve abrir `index.html` con `file://` — el navegador bloquea el `fetch` que usa el jugador para cargar `game.aigs`.
2. **Subirlo a GitHub Pages** (gratis, sin cuenta adicional si ya usas GitHub):
   - Copia el contenido de `dist/mi-juego/` a una rama `gh-pages` (o a la carpeta `docs/` de un repo, según cómo tengas configurado Pages).
   - Actívalo en el repo: **Settings → Pages → Source**.
   - Tu juego queda en `https://<usuario>.github.io/<repo>/`.
3. **Cualquier otro hosting estático** (Netlify, Vercel, Cloudflare Pages, un servidor propio con nginx…) funciona igual: es contenido 100% estático, solo hace falta servirlo por HTTP(S).
4. **HTTPS recomendado:** varios navegadores solo dan WebGPU (no WebGL) en contextos seguros (`https://` o `localhost`). GitHub Pages, Netlify, Vercel y Cloudflare Pages ya sirven todo por HTTPS por defecto.

---

## Android (Google Play o instalación directa)

### Instalación directa (sin tienda)

Ya lo hiciste si seguiste la guía de M15: `aigs export --target android`, luego `adb install ruta/al/juego.apk`. Sirve para probar en tu propio dispositivo o repartir el `.apk` directamente, pero Android avisará al usuario que es una fuente "no confiable" salvo que lo publiques en una tienda.

### Publicarlo en Google Play

1. **Cuenta de desarrollador de Google Play** (pago único, gestionado por Google — no por este proyecto): [play.google.com/console/signup](https://play.google.com/console/signup).
2. **Genera tu propio keystore de release** (nunca uses el de depuración autogenerado para publicar):
   ```bash
   keytool -genkey -v -keystore mi-juego-release.keystore \
     -alias mi-juego -keyalg RSA -keysize 2048 -validity 10000
   ```
   Guarda el archivo y la contraseña en un sitio seguro — **si los pierdes no podrás actualizar la app publicada nunca más**.
3. **Configura el firmado** en `exporters/android-player/Cargo.toml` (o en tu copia de la plantilla), en `[package.metadata.android.signing.release]`, apuntando a ese keystore (consulta la documentación de [`cargo-apk`](https://github.com/rust-mobile/cargo-apk) para el formato exacto de esta sección — cambia entre versiones).
4. **Exporta en modo release:**
   ```bash
   aigs export mi-juego/game.aigs --target android --output dist --release
   ```
5. **Sube el `.apk`** (o compílalo como `.aab` si tu versión de `cargo-apk` lo soporta — Play exige Android App Bundle para apps nuevas) en Play Console: **Producción → Crear versión**.
6. Completa la ficha de la tienda (capturas, descripción, clasificación de contenido) — esto es 100% manual, cada tienda tiene sus propios requisitos y revisión.

---

## Desktop

No hay "tienda" única — se distribuye la carpeta/`.zip` de `aigs export --target desktop` directamente (tu propia web, itch.io, Steam si más adelante se integra, etc.). itch.io en particular acepta subir el `.zip` tal cual y sirve de "tienda" ligera sin pasos adicionales de firmado.

---

## Checklist antes de publicar

- [ ] Probado localmente el artefacto exportado (no solo `aigs run` en el proyecto original).
- [ ] Web: servido por HTTPS, verificado en al menos Chrome/Firefox.
- [ ] Android: keystore de **release** propio (no el de depuración), guardado de forma segura.
- [ ] `docs/plan.md` / `ROADMAP.md` actualizados si esto forma parte de una release del propio AI Game Studio.
