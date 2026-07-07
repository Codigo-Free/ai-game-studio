# AI Game Studio

<p align="center"><img src="logo.png" alt="AI Game Studio" width="200"></p>

> **Build Games at the Speed of Imagination.**

[![CI](https://github.com/Codigo-Free/ai-game-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/Codigo-Free/ai-game-studio/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Codigo-Free/ai-game-studio?include_prereleases&label=release)](https://github.com/Codigo-Free/ai-game-studio/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**AI Game Studio** is an **AI-First**, **open source** 2D game development platform that combines the productivity of Adobe Flash's visual paradigm (timeline, keyframes, direct editing) with modern architectures (Rust, ECS, WGPU) and integrates Artificial Intelligence as an active member of the development team.

> **The AI knows the game, and the game knows the AI.**

## Project status

🎉 **0.4.0 released** (Phases 1–2 complete, Phase 3 closed over Desktop/Web/Android, Phase 4 — deep AI — complete, milestones M0–M21). The editor's Chat can now answer questions, propose and apply reviewable changes, coordinate specialized agents, and generate a whole game across several scenes from a natural-language instruction. See the [CHANGELOG](CHANGELOG.md) and the [roadmap](ROADMAP.md).

## Get started in 3 minutes

```bash
git clone https://github.com/Codigo-Free/ai-game-studio && cd ai-game-studio
cargo install --path cli
aigs run examples/robot-rescue/game.aigs        # play the demo
cd editor && npm install && npm run tauri dev   # open the editor
```

Follow the **[quick start guide](docs/guia-inicio.md)** for a tour of the editor and your first game in 8 steps. Editor installers and CLI binaries are available in [Releases](https://github.com/Codigo-Free/ai-game-studio/releases) — ⚠️ on Wayland, prefer the `.deb`/`.rpm` installer over the AppImage, which can fail with an `EGL_BAD_ALLOC` error on some GPUs (confirmed AppImage-packaging-specific; see the [known issue](docs/guia-inicio.md#1-requisitos)).

## What works today (0.4.0)

- 🎨 **Visual editor** (Tauri 2 + React): viewport with drag/zoom/pan, scene tree, inspector, asset browser with import, live console with metrics, global undo/redo, and Desktop/Web/Android export from a dropdown.
- 🎞 **Flash-style timeline**: per-entity/property tracks, draggable keyframes with easing, scrubbing and playback with viewport preview.
- 🕹 **Code-free behaviors**: "when *key/click/scene start/animation end/collision* → *move / go to scene / play animation/sound / emit particles*", edited from the inspector.
- ⚙️ **Custom Rust runtime**: ECS with generational indices, instanced 2D rendering over WGPU, 60 Hz game loop with interpolated rendering, multi-scene. Querying 10k entities costs ~21 µs ([benchmarks](docs/testing.md)).
- 🧱 **2D physics** (rapier2d), **audio** (kira), **spritesheets and animation state machines**, and **particles** as ECS entities.
- 📜 **User scripting** (sandboxed rhai) with persistent state **across scenes and across real play sessions** (`save.json`, autosave) — see the [Tamagotchi](examples/tamagotchi/) demo.
- 📦 **Export to Desktop, Web and Android**: `aigs export --target desktop|web|android` — the same project runs natively, in the browser (WebGPU/WebGL) or as a signed APK (Vulkan), unchanged.
- 👆 **Touch input**: a finger behaves like the mouse, and the `virtual_button` component simulates on-screen keys for keyboard-driven games.
- 📄 **AI-Ready `.aigs` format**: the entire game is readable, versionable JSON — the foundation for AI to create and modify complete games ([specification](sdk/aigs-format/SPEC.md)).
- 🤖 **AI Chat, four modes**: ask questions about the open project; propose a reviewable change (entities/scripts) with one-click apply/undo; orchestrate specialized agents (Architect, Level Designer, Programmer, Physics, Audio, Animator) for a higher-level instruction; or generate a whole game across several scenes from a single natural-language prompt — all backed by a local Ollama model or Claude.
- 🔧 **`aigs` CLI**: `validate`, `run`, `export --target ...` and `script-api` to work without opening the editor.

## What's next

- **Phase 3**: iOS exporter (M16), deferred with no date — needs macOS + Xcode.
- **Phase 5**: plugin SDK, marketplace, collaboration.

## Examples

Each folder has its own `README.md` with instructions and controls.

| Project | What it demonstrates |
|---|---|
| [`examples/robot-rescue/`](examples/robot-rescue/) | **Complete demo game**: animated menu, playable level (arrows + click), a scripted drone and a win screen chained via `animation_end`. |
| [`examples/tamagotchi/`](examples/tamagotchi/) | **Persistent virtual pet** (M13): stats that decay in real time even while the game is closed, cared for with keys 1/2/3, a single script. |
| [`examples/physics-playground/`](examples/physics-playground/) | 2D physics: falling crates, a bouncing ball, a robot pushing crates, a sensor with particles. |
| [`examples/hello-world/`](examples/hello-world/) | Minimal project: scenes, animations and basic behaviors. |
| [`examples/bouncing-sprites/`](examples/bouncing-sprites/) | Using the runtime as a Rust library, no editor (hundreds of sprites at 60 FPS). |

## Monorepo structure

| Directory | Contents |
|---|---|
| [`docs/`](docs/) | Design documentation: vision, architecture, plan, quick start guide, decisions. |
| [`editor/`](editor/) | Visual editor — Tauri 2 + React + TypeScript. |
| [`runtime/`](runtime/) | Engine — Rust crates: `aigs-ecs`, `aigs-render`, `aigs-anim`, `aigs-project`, `aigs-runtime`. |
| [`cli/`](cli/) | `aigs` binary (`validate`, `run`, `export`). |
| [`exporters/`](exporters/) | Per-platform exporters (Phase 2+). |
| [`sdk/`](sdk/) | Public contract: `.aigs` format specification and extension APIs. |
| [`examples/`](examples/) | Example games and demos. |
| [`tests/`](tests/) | Full-system integration tests. |

## Documentation

- [Quick start guide](docs/guia-inicio.md) — install, play the demo and build your first game.
- [Full project overview](docs/proyecto.md) — what it is, why it exists, philosophy and goals.
- [Master plan](docs/plan.md) — MVP milestones (M0–M6) and later phases.
- [Architecture](docs/arquitectura.md) — modules, `.aigs` format, decisions table.
- [Vision](docs/vision.md) · [Editor](docs/editor.md) · [Runtime](docs/runtime.md) · [AI](docs/ia.md) · [Exporters](docs/exportadores.md) · [Plugins](docs/plugins.md) · [Testing](docs/testing.md) · [CI/CD](docs/ci-cd.md) · [Publishing guide](docs/guia-publicacion.md)

Most of `docs/` is written in Spanish (the project's primary working language); code, identifiers and commit messages are in English.

## Technologies

**Rust** · **WGPU** · **Tauri 2** · **React** · **TypeScript** · **Ollama / Claude / GPT / Gemini** (Phase 4)

## Contributing

The project is open to the community. Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started. Development follows an **AI First** methodology: this project is designed, implemented, tested and documented in collaboration with AI, with full traceability in [CLAUDE.md](CLAUDE.md).

## Authorship

Created by **efrasoft@gmail.com** on **HarnessOS** with **Visual Studio Code** and **Claude**.

## License

[MIT](LICENSE)
