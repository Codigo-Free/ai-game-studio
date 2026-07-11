// Instanced sprite shader: one quad (6 vertices) per instance, transformed
// on the GPU from the instance's center/half-size/rotation.

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct InstanceInput {
    @location(0) center: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) rotation: f32,
    @location(3) opacity: f32,
    // Sub-rectangle of the texture to sample (spritesheet frame).
    @location(4) uv_rect: vec4<f32>, // (u0, v0, u1, v1)
    // Multiplies the sampled color; [1,1,1,1] for a normal, untinted sprite.
    @location(5) tint: vec4<f32>,
    // > 0.5 masks the quad to an inscribed circle (`shape` primitives).
    @location(6) shape_kind: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    // Unrotated quad-local position in -1..1, for the circle mask test.
    @location(2) local: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) shape_kind: f32,
};

const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    let corner = CORNERS[vertex_index];
    let local = corner * instance.half_size;
    let cos_r = cos(instance.rotation);
    let sin_r = sin(instance.rotation);
    let world = vec2<f32>(
        local.x * cos_r - local.y * sin_r,
        local.x * sin_r + local.y * cos_r,
    ) + instance.center;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world, 0.0, 1.0);
    let unit_u = (corner.x + 1.0) * 0.5;
    let unit_v = 1.0 - (corner.y + 1.0) * 0.5;
    out.uv = vec2<f32>(
        mix(instance.uv_rect.x, instance.uv_rect.z, unit_u),
        mix(instance.uv_rect.y, instance.uv_rect.w, unit_v),
    );
    out.opacity = instance.opacity;
    out.local = corner;
    out.tint = instance.tint;
    out.shape_kind = instance.shape_kind;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.shape_kind > 0.5 && dot(in.local, in.local) > 1.0) {
        discard;
    }
    let color = textureSample(sprite_texture, sprite_sampler, in.uv);
    return vec4<f32>(color.rgb * in.tint.rgb, color.a * in.opacity * in.tint.a);
}
