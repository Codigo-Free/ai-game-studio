//! Scene instantiation (milestone M2): turns authored `.aigs` scene data
//! into live ECS entities.
//!
//! Child transforms are composed into world space at instantiation time
//! (static flattening). A live parent/child hierarchy with transform
//! propagation arrives with the editor scene tree in milestone M3.

use std::collections::HashMap;

use aigs_ecs::{Entity, World};
use aigs_project::{EntityNode, Scene};

use crate::assets::{AssetStore, TextureInfo};
use crate::components::{
    Camera2D, Collider2DShape, Name, RigidBody2D, Sprite, Transform2D, Visibility,
};

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("entity \"{entity}\" references unknown image asset \"{asset}\"")]
    UnknownAsset { entity: String, asset: String },
    #[error("duplicated entity id \"{0}\" in scene")]
    DuplicatedId(String),
}

/// Maps authored entity ids to live ECS entities, for animation tracks (M4)
/// and editor selection (M3).
#[derive(Debug, Default)]
pub struct SceneInstance {
    by_id: HashMap<String, Entity>,
}

impl SceneInstance {
    pub fn entity(&self, id: &str) -> Option<Entity> {
        self.by_id.get(id).copied()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Resolves an asset id to a loaded texture. Implemented by [`AssetStore`];
/// tests can inject a fake resolver and skip the GPU entirely.
pub trait ResolveTexture {
    fn resolve(&self, asset_id: &str) -> Option<TextureInfo>;
}

impl ResolveTexture for AssetStore {
    fn resolve(&self, asset_id: &str) -> Option<TextureInfo> {
        self.texture(asset_id)
    }
}

/// Spawns every entity of `scene` into `world`. Returns the id map.
pub fn instantiate_scene(
    world: &mut World,
    scene: &Scene,
    textures: &impl ResolveTexture,
) -> Result<SceneInstance, SceneError> {
    let mut instance = SceneInstance::default();
    for node in &scene.entities {
        spawn_node(world, node, Transform2D::default(), textures, &mut instance)?;
    }
    Ok(instance)
}

fn spawn_node(
    world: &mut World,
    node: &EntityNode,
    parent: Transform2D,
    textures: &impl ResolveTexture,
    instance: &mut SceneInstance,
) -> Result<(), SceneError> {
    let local = node.components.transform2d.clone().unwrap_or_default();
    let world_transform = compose(parent, local);

    let entity = world.spawn();
    if instance.by_id.insert(node.id.clone(), entity).is_some() {
        return Err(SceneError::DuplicatedId(node.id.clone()));
    }
    world.insert(entity, Name(node.name.clone()));
    world.insert(entity, Visibility::default());
    world.insert(entity, world_transform);

    if let Some(sprite) = &node.components.sprite {
        let texture = textures
            .resolve(&sprite.asset)
            .ok_or_else(|| SceneError::UnknownAsset {
                entity: node.id.clone(),
                asset: sprite.asset.clone(),
            })?;
        world.insert(
            entity,
            Sprite {
                texture: texture.id,
                width: sprite.width.unwrap_or(texture.width),
                height: sprite.height.unwrap_or(texture.height),
                opacity: sprite.opacity,
                layer: sprite.layer,
            },
        );
    }

    if let Some(camera) = &node.components.camera2d {
        world.insert(entity, Camera2D { zoom: camera.zoom });
    }

    if let Some(body) = &node.components.rigidbody2d {
        world.insert(
            entity,
            RigidBody2D {
                body: body.body,
                gravity_scale: body.gravity_scale,
                vx: body.vx,
                vy: body.vy,
                fixed_rotation: body.fixed_rotation,
            },
        );
    }

    if let Some(collider) = &node.components.collider2d {
        // Collider defaults derive from the sprite's visible size.
        let (visible_w, visible_h) = world
            .get::<Sprite>(entity)
            .map(|sprite| {
                (
                    sprite.width * world_transform.scale_x.abs(),
                    sprite.height * world_transform.scale_y.abs(),
                )
            })
            .unwrap_or((32.0, 32.0));
        let width = collider.width.unwrap_or(visible_w);
        let height = collider.height.unwrap_or(visible_h);
        world.insert(
            entity,
            Collider2DShape {
                shape: collider.shape,
                half_width: width / 2.0,
                half_height: height / 2.0,
                radius: collider.radius.unwrap_or_else(|| width.max(height) / 2.0),
                sensor: collider.sensor,
                restitution: collider.restitution,
                friction: collider.friction,
            },
        );
    }

    for child in &node.children {
        spawn_node(world, child, world_transform, textures, instance)?;
    }
    Ok(())
}

/// Composes a child transform with its parent (2D TRS composition).
fn compose(parent: Transform2D, local: aigs_project::Transform2D) -> Transform2D {
    let radians = parent.rotation.to_radians();
    let (sin, cos) = radians.sin_cos();
    let scaled_x = local.x * parent.scale_x;
    let scaled_y = local.y * parent.scale_y;
    Transform2D {
        x: parent.x + scaled_x * cos - scaled_y * sin,
        y: parent.y + scaled_x * sin + scaled_y * cos,
        rotation: parent.rotation + local.rotation,
        scale_x: parent.scale_x * local.scale_x,
        scale_y: parent.scale_y * local.scale_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aigs_render::TextureId;

    struct FakeTextures;

    impl ResolveTexture for FakeTextures {
        fn resolve(&self, asset_id: &str) -> Option<TextureInfo> {
            (asset_id == "hero").then_some(TextureInfo {
                id: TextureId::default(),
                width: 64.0,
                height: 32.0,
            })
        }
    }

    fn scene_from_json(json: &str) -> Scene {
        Scene::from_json(json).expect("valid scene JSON")
    }

    #[test]
    fn instantiates_entities_with_components() {
        let scene = scene_from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "main",
                "entities": [
                    {
                        "id": "camera", "name": "Camera",
                        "components": { "transform2d": {}, "camera2d": { "zoom": 2.0 } }
                    },
                    {
                        "id": "hero", "name": "Hero",
                        "components": {
                            "transform2d": { "x": 10.0, "y": 20.0 },
                            "sprite": { "asset": "hero", "opacity": 0.5, "layer": 3 }
                        }
                    }
                ]
            }"#,
        );
        let mut world = World::new();
        let instance = instantiate_scene(&mut world, &scene, &FakeTextures).unwrap();

        assert_eq!(instance.len(), 2);
        let hero = instance.entity("hero").unwrap();
        let transform = world.get::<Transform2D>(hero).unwrap();
        assert_eq!((transform.x, transform.y), (10.0, 20.0));
        let sprite = world.get::<Sprite>(hero).unwrap();
        // Size defaults to the texture dimensions.
        assert_eq!((sprite.width, sprite.height), (64.0, 32.0));
        assert_eq!(sprite.opacity, 0.5);
        assert_eq!(sprite.layer, 3);
        let camera = instance.entity("camera").unwrap();
        assert_eq!(world.get::<Camera2D>(camera).unwrap().zoom, 2.0);
        assert_eq!(world.get::<Name>(hero).unwrap().0, "Hero");
    }

    #[test]
    fn children_are_flattened_into_world_space() {
        let scene = scene_from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "main",
                "entities": [{
                    "id": "parent", "name": "Parent",
                    "components": { "transform2d": { "x": 100.0, "y": 0.0, "scale_x": 2.0, "scale_y": 2.0 } },
                    "children": [{
                        "id": "child", "name": "Child",
                        "components": { "transform2d": { "x": 5.0, "y": 0.0 } }
                    }]
                }]
            }"#,
        );
        let mut world = World::new();
        let instance = instantiate_scene(&mut world, &scene, &FakeTextures).unwrap();
        let child = instance.entity("child").unwrap();
        let transform = world.get::<Transform2D>(child).unwrap();
        assert_eq!(transform.x, 110.0, "child offset scaled by parent");
        assert_eq!(transform.scale_x, 2.0, "scale inherited");
    }

    #[test]
    fn unknown_asset_is_an_error() {
        let scene = scene_from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "main",
                "entities": [{
                    "id": "e", "name": "E",
                    "components": { "sprite": { "asset": "missing" } }
                }]
            }"#,
        );
        let mut world = World::new();
        let result = instantiate_scene(&mut world, &scene, &FakeTextures);
        assert!(matches!(result, Err(SceneError::UnknownAsset { .. })));
    }

    #[test]
    fn duplicated_ids_are_rejected() {
        let scene = scene_from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "main",
                "entities": [
                    { "id": "same", "name": "A", "components": {} },
                    { "id": "same", "name": "B", "components": {} }
                ]
            }"#,
        );
        let mut world = World::new();
        let result = instantiate_scene(&mut world, &scene, &FakeTextures);
        assert!(matches!(result, Err(SceneError::DuplicatedId(_))));
    }
}
