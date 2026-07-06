//! Window runner: fixed-timestep simulation with interpolated rendering.

use std::sync::Arc;
use std::time::Instant;

use aigs_ecs::World;
use aigs_render::{CameraView, Color, RenderError, Renderer, SpriteInstance, SurfaceError};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::components::{Camera2D, PrevTransform2D, Sprite, Transform2D, Visibility};
use crate::input::Input;
use crate::time::Time;

/// Simulation tick rate: 60 updates per second.
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// Cap on frame time to avoid the spiral of death after a long stall.
const MAX_FRAME_TIME: f32 = 0.25;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error(transparent)]
    Render(#[from] RenderError),
}

/// Window and loop configuration.
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub clear_color: Color,
    /// Appends a live FPS counter to the window title.
    pub show_fps_in_title: bool,
    /// Exits after rendering this many frames (used by smoke tests).
    pub max_frames: Option<u64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "AI Game Studio".to_string(),
            width: 1280,
            height: 720,
            clear_color: Color::rgba(0.08, 0.08, 0.12, 1.0),
            show_fps_in_title: true,
            max_frames: None,
        }
    }
}

type SetupFn = Box<dyn FnOnce(&mut World, &mut Renderer)>;
type UpdateFn = Box<dyn FnMut(&mut World, &Time, &Input)>;

/// Opens a window and runs the game loop until the window closes.
///
/// `setup` runs once with the renderer available (create textures, spawn
/// entities); `update` runs at a fixed 60 Hz simulation rate.
pub fn run(
    config: AppConfig,
    setup: impl FnOnce(&mut World, &mut Renderer) + 'static,
    update: impl FnMut(&mut World, &Time, &Input) + 'static,
) -> Result<(), RunError> {
    let event_loop = EventLoop::new()?;
    let app = App {
        config,
        world: World::new(),
        setup: Some(Box::new(setup)),
        update: Box::new(update),
        window: None,
        renderer: None,
        #[cfg(target_arch = "wasm32")]
        pending_renderer: None,
        input: Input::default(),
        time: Time::default(),
        accumulator: 0.0,
        last_instant: None,
        frames_rendered: 0,
        fps_frames: 0,
        fps_since: None,
        init_error: None,
    };
    run_event_loop(event_loop, app)
}

/// Desktop: `run_app` blocks the calling thread until the window closes,
/// then hands back whatever `setup` failed with, if anything.
#[cfg(not(target_arch = "wasm32"))]
fn run_event_loop(event_loop: EventLoop<()>, mut app: App) -> Result<(), RunError> {
    event_loop.run_app(&mut app)?;
    match app.init_error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

/// Web: a browser tab can never block its own JS thread, so `spawn_app`
/// hands the app to the browser's own event loop and returns immediately —
/// there is no "final result" to report here. Setup/render errors are
/// surfaced through `eprintln!` (routed to the console, see `Renderer`),
/// not through this function's return value.
#[cfg(target_arch = "wasm32")]
fn run_event_loop(event_loop: EventLoop<()>, app: App) -> Result<(), RunError> {
    use winit::platform::web::EventLoopExtWebSys;
    event_loop.spawn_app(app);
    Ok(())
}

/// Not yet resolved, resolved (ok or error). Only used on Web, where GPU
/// adapter/device setup is unavoidably asynchronous (see
/// [`aigs_render::Renderer::new_async`]) and `resumed()` — a synchronous
/// winit callback — can't block the browser's one JS thread waiting on it.
#[cfg(target_arch = "wasm32")]
type PendingRenderer = std::rc::Rc<std::cell::RefCell<Option<Result<Renderer, RenderError>>>>;

struct App {
    config: AppConfig,
    world: World,
    setup: Option<SetupFn>,
    update: UpdateFn,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    #[cfg(target_arch = "wasm32")]
    pending_renderer: Option<PendingRenderer>,
    input: Input,
    time: Time,
    accumulator: f32,
    last_instant: Option<Instant>,
    frames_rendered: u64,
    fps_frames: u32,
    fps_since: Option<Instant>,
    init_error: Option<RenderError>,
}

impl App {
    /// On Web, promotes a finished async renderer into `self.renderer` and
    /// runs the deferred `setup`. A no-op on Desktop, where `resumed()`
    /// already did this synchronously.
    fn try_finish_init(
        &mut self,
        #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
        event_loop: &ActiveEventLoop,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(pending) = &self.pending_renderer else {
                return;
            };
            let Some(result) = pending.borrow_mut().take() else {
                return; // still awaiting the adapter/device
            };
            self.pending_renderer = None;
            match result {
                Ok(mut renderer) => {
                    let viewport = renderer.viewport();
                    self.input
                        .set_viewport(viewport.width as f32, viewport.height as f32);
                    if let Some(setup) = self.setup.take() {
                        setup(&mut self.world, &mut renderer);
                    }
                    self.renderer = Some(renderer);
                }
                Err(error) => {
                    eprintln!("failed to initialize renderer: {error}");
                    self.init_error = Some(error);
                    event_loop.exit();
                }
            }
        }
    }

    fn tick(&mut self, event_loop: &ActiveEventLoop) {
        self.try_finish_init(event_loop);
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let now = Instant::now();
        let frame_dt = self
            .last_instant
            .map(|last| (now - last).as_secs_f32().min(MAX_FRAME_TIME))
            .unwrap_or(FIXED_DT);
        self.last_instant = Some(now);
        self.accumulator += frame_dt;

        while self.accumulator >= FIXED_DT {
            snapshot_prev_transforms(&mut self.world);
            self.time.delta = FIXED_DT;
            (self.update)(&mut self.world, &self.time, &self.input);
            self.input.end_tick();
            self.time.elapsed += f64::from(FIXED_DT);
            self.time.tick += 1;
            self.accumulator -= FIXED_DT;
        }

        let alpha = self.accumulator / FIXED_DT;
        let camera = extract_camera(&self.world);
        let mut sprites = extract_sprites(&self.world, alpha);
        match renderer.render(self.config.clear_color, camera, &mut sprites) {
            Ok(()) => {}
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                let viewport = renderer.viewport();
                renderer.resize(viewport.width, viewport.height);
            }
            Err(SurfaceError::OutOfMemory) => {
                eprintln!("render error: out of GPU memory, exiting");
                event_loop.exit();
            }
            Err(error) => eprintln!("render error: {error}"),
        }

        self.frames_rendered += 1;
        if let Some(max) = self.config.max_frames {
            if self.frames_rendered >= max {
                event_loop.exit();
            }
        }
        self.measure_fps(now);
    }

    fn measure_fps(&mut self, now: Instant) {
        self.fps_frames += 1;
        let since = *self.fps_since.get_or_insert(now);
        let elapsed = (now - since).as_secs_f32();
        if elapsed >= 1.0 {
            self.time.fps = self.fps_frames as f32 / elapsed;
            self.fps_frames = 0;
            self.fps_since = Some(now);
            if self.config.show_fps_in_title {
                if let Some(window) = &self.window {
                    window.set_title(&format!(
                        "{} — {:.0} FPS — {} entities",
                        self.config.title,
                        self.time.fps,
                        self.world.len()
                    ));
                }
            }
        }
    }

    /// Desktop: builds the renderer synchronously (blocks on GPU setup) and
    /// runs `setup` immediately, exactly as before this platform ever had
    /// a Web target.
    #[cfg(not(target_arch = "wasm32"))]
    fn init_renderer(&mut self, window: Arc<Window>) {
        match Renderer::new(window) {
            Ok(mut renderer) => {
                let viewport = renderer.viewport();
                self.input
                    .set_viewport(viewport.width as f32, viewport.height as f32);
                if let Some(setup) = self.setup.take() {
                    setup(&mut self.world, &mut renderer);
                }
                self.renderer = Some(renderer);
            }
            Err(error) => self.init_error = Some(error),
        }
    }

    /// Web: `resumed()` can't block on the GPU adapter/device (there's no
    /// way to synchronously wait on a JS Promise from the browser's only
    /// thread), so it kicks off the async setup and returns; a later
    /// [`App::try_finish_init`] call picks up the result once ready.
    #[cfg(target_arch = "wasm32")]
    fn init_renderer(&mut self, window: Arc<Window>) {
        let slot: PendingRenderer = std::rc::Rc::new(std::cell::RefCell::new(None));
        self.pending_renderer = Some(slot.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result = Renderer::new_async(window).await;
            *slot.borrow_mut() = Some(result);
        });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));
        #[cfg(target_arch = "wasm32")]
        let attributes = {
            use winit::platform::web::WindowAttributesExtWebSys;
            // winit creates the <canvas> in memory; nothing shows up on the
            // page unless it's told to append it to the DOM itself.
            attributes.with_append(true)
        };
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self.init_renderer(window);
        if self.init_error.is_some() {
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.input
                    .set_viewport(size.width as f32, size.height as f32);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.input.on_key_event(&event),
            WindowEvent::CursorMoved { position, .. } => self
                .input
                .set_mouse_position(position.x as f32, position.y as f32),
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.on_mouse_button(state, button)
            }
            WindowEvent::RedrawRequested => self.tick(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Tear down GPU resources while the event loop (and its Wayland/X11
        // connection) is still alive; dropping the WGPU instance after the
        // display connection closes segfaults on Wayland + EGL (Mesa).
        self.renderer = None;
        self.window = None;
    }
}

/// Copies every `Transform2D` into its `PrevTransform2D` before a tick.
fn snapshot_prev_transforms(world: &mut World) {
    let mut snapshots = Vec::new();
    world.for_each::<Transform2D>(|entity, transform| snapshots.push((entity, *transform)));
    for (entity, transform) in snapshots {
        world.insert(entity, PrevTransform2D(transform));
    }
}

fn extract_camera(world: &World) -> CameraView {
    let mut camera = CameraView::default();
    let mut found = false;
    world.for_each2::<Transform2D, Camera2D>(|_, transform, cam| {
        if !found {
            camera = CameraView {
                x: transform.x,
                y: transform.y,
                zoom: cam.zoom,
            };
            found = true;
        }
    });
    camera
}

/// Builds the sprite list for this frame, interpolating between the previous
/// and current simulation states with factor `alpha`.
fn extract_sprites(world: &World, alpha: f32) -> Vec<SpriteInstance> {
    let mut sprites = Vec::new();
    world.for_each2::<Transform2D, Sprite>(|entity, transform, sprite| {
        if let Some(visibility) = world.get::<Visibility>(entity) {
            if !visibility.0 {
                return;
            }
        }
        let (x, y, rotation) = match world.get::<PrevTransform2D>(entity) {
            Some(prev) => (
                lerp(prev.0.x, transform.x, alpha),
                lerp(prev.0.y, transform.y, alpha),
                lerp(prev.0.rotation, transform.rotation, alpha),
            ),
            None => (transform.x, transform.y, transform.rotation),
        };
        sprites.push(SpriteInstance {
            x,
            y,
            rotation: -rotation.to_radians(),
            half_width: sprite.width * transform.scale_x / 2.0,
            half_height: sprite.height * transform.scale_y / 2.0,
            opacity: sprite.opacity,
            layer: sprite.layer,
            texture: sprite.texture,
            uv: sprite.uv(),
        });
    });
    sprites
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
