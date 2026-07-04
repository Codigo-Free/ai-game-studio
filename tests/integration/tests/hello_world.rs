//! Loads the real `examples/hello-world` project and instantiates its
//! initial scene into a live ECS world (milestone M2 acceptance test).

use std::path::{Path, PathBuf};

use aigs_project::{Project, Scene};
use aigs_runtime::{
    instantiate_scene, Camera2D, Name, ResolveTexture, Sprite, TextureInfo, Transform2D, World,
};

fn hello_world_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-world")
}

/// Resolves every image asset declared in the manifest, faking the GPU
/// upload but keeping the real files on disk honest.
struct DiskBackedFake {
    ids: Vec<String>,
}

impl ResolveTexture for DiskBackedFake {
    fn resolve(&self, asset_id: &str) -> Option<TextureInfo> {
        self.ids
            .iter()
            .any(|id| id == asset_id)
            .then_some(TextureInfo {
                id: Default::default(),
                width: 64.0,
                height: 64.0,
                sheet: None,
                frame_width: 64.0,
                frame_height: 64.0,
            })
    }
}

#[test]
fn hello_world_project_instantiates_into_a_world() {
    let root = hello_world_root();
    let project = Project::load(&root.join("game.aigs")).expect("manifest loads");
    assert_eq!(project.name, "Hello World");
    assert!(
        project.scenes.contains(&project.initial_scene),
        "initial scene must be listed"
    );

    // Every declared asset file must exist on disk.
    let mut ids = Vec::new();
    for asset in &project.assets {
        assert!(
            root.join(&asset.path).is_file(),
            "asset file {} missing",
            asset.path
        );
        ids.push(asset.id.clone());
    }

    let scene = Scene::load(&root.join(&project.initial_scene)).expect("scene loads");
    let mut world = World::new();
    let instance =
        instantiate_scene(&mut world, &scene, &DiskBackedFake { ids }).expect("scene instantiates");

    // Scene contents: camera + hero + shadow child + two moons.
    assert_eq!(instance.len(), 5);
    assert_eq!(world.len(), 5);

    let camera = instance.entity("camera").expect("camera exists");
    assert!(world.get::<Camera2D>(camera).is_some());

    let hero = instance.entity("hero").expect("hero exists");
    assert_eq!(world.get::<Name>(hero).unwrap().0, "Hero");
    let hero_sprite = world.get::<Sprite>(hero).unwrap();
    assert_eq!(
        (hero_sprite.width, hero_sprite.height),
        (64.0, 64.0),
        "sprite size defaults to texture size"
    );

    // The shadow is a child of the hero: its transform must be composed
    // into world space (hero at origin, scale 2 => child offset doubled).
    let shadow = instance.entity("hero-shadow").expect("shadow exists");
    let shadow_transform = world.get::<Transform2D>(shadow).unwrap();
    assert_eq!(
        (shadow_transform.x, shadow_transform.y),
        (20.0, -20.0),
        "child offset scaled by parent scale"
    );

    // Explicit size override in the scene file wins over the texture size.
    let moon = instance.entity("moon-right").expect("moon exists");
    let moon_sprite = world.get::<Sprite>(moon).unwrap();
    assert_eq!((moon_sprite.width, moon_sprite.height), (48.0, 48.0));
}
