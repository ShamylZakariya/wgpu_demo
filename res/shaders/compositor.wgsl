
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) view_dir: vec3<f32>, // direction in world space from camera to fragment
};

struct CompositorUniform {
    // x: z_near, y: z_far, z: width in pixels, w: height in pixels
    @location(0) camera_z_near_far_width_height: vec4<f32>,
}

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    proj_inverse: mat4x4<f32>,
    view_inverse: mat4x4<f32>,
};

@group(0) @binding(0)
var color_attachment_texture: texture_2d<f32>;

@group(0) @binding(1)
var color_attachment_sampler: sampler;

@group(0) @binding(2)
var depth_attachment_texture: texture_2d<f32>;

@group(0) @binding(3)
var depth_attachment_sampler: sampler;

@group(0) @binding(4)
var environment_map_texture: texture_cube<f32>;

@group(0) @binding(5)
var environment_map_sampler: sampler;


@group(1) @binding(0)
var<uniform> compositor: CompositorUniform;

@group(2) @binding(0)
var<uniform> camera: CameraUniform;

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    // https://github.com/hughsk/glsl-hsv2rgb/blob/master/index.glsl
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), hsv.y);
}

@vertex
fn compositor_vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    // wgsl doesn't let us index `let` arrays with a variable. So it has to be a `var` local to this function.
    var fsq_clip_positions:array<vec4<f32>,3> = array<vec4<f32>, 3>(vec4<f32>(-1.0, 1.0, 0.0, 1.0), vec4<f32>(3.0, 1.0, 0.0, 1.0), vec4<f32>(-1.0, -3.0, 0.0, 1.0));
    var fsq_tex_coords:array<vec2<f32>,3> = array<vec2<f32>, 3>(vec2<f32>(0.0, 0.0), vec2<f32>(2.0, 0.0), vec2<f32>(0.0, 2.0));

    var out: VertexOutput;
    out.tex_coord = fsq_tex_coords[in_vertex_index];
    out.clip_position = fsq_clip_positions[in_vertex_index];

    // compute direction to fragment in world space
    let unprojected = camera.proj_inverse * out.clip_position;
    out.view_dir = (camera.view_inverse * vec4<f32>(unprojected.xyz, 0.0)).xyz;

    return out;
}

// Samples the rendered scene, adding the sky environment
fn scene(in: VertexOutput) -> vec4<f32> {
    var color = textureSample(color_attachment_texture, color_attachment_sampler, in.tex_coord);
    let depth = textureSample(depth_attachment_texture, depth_attachment_sampler, in.tex_coord).r;
    let sky_color = textureSampleBias(environment_map_texture, environment_map_sampler, normalize(in.view_dir), 0.0);

    if (depth < 1.0) {
        return color;
    } else {
        return sky_color;
    }
}

// linear depth of scene, normalized to [0,1]
fn normalized_linear_depth(in: VertexOutput) -> f32 {
    let depth = textureSample(depth_attachment_texture, depth_attachment_sampler, in.tex_coord).r;
    let z_near = compositor.camera_z_near_far_width_height.x;
    let z_far = compositor.camera_z_near_far_width_height.y;
    return (z_near + (pow(z_far + 1.0, depth) - 1.0)) / z_far;
}

// linear depth of scene in world [z_near, z_far]
fn world_linear_depth(in: VertexOutput) -> f32 {
    let depth = textureSample(depth_attachment_texture, depth_attachment_sampler, in.tex_coord).r;
    let z_near = compositor.camera_z_near_far_width_height.x;
    let z_far = compositor.camera_z_near_far_width_height.y;
    return z_near + (pow(z_far + 1.0, depth) - 1.0);
}

@fragment
fn compositor_fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return scene(in);
}