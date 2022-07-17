
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct CompositorUniform {
    // x: z_near, y: z_far, z: width in pixels, w: height in pixels
    @location(0) camera_z_near_far_width_height: vec4<f32>,
}

@group(0) @binding(0)
var t_color_attachment: texture_2d<f32>;

@group(0) @binding(1)
var s_color_attachment: sampler;

@group(0) @binding(2)
var t_depth_attachment: texture_2d<f32>;

@group(0) @binding(3)
var s_depth_attachment: sampler;

@group(1) @binding(0)
var<uniform> compositor: CompositorUniform;

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    // https://github.com/hughsk/glsl-hsv2rgb/blob/master/index.glsl
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), hsv.y);
}

fn linearize_depth(depth: f32, z_near: f32, z_far: f32) -> f32 {
    return (pow(z_far + 1.0, depth) - 1.0) / z_far;
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
    return out;
}

@fragment
fn compositor_fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Output color
    return textureSample(t_color_attachment, s_color_attachment, in.tex_coord);

    // let depth = textureSample(t_depth_attachment, s_depth_attachment, in.tex_coord).r;
    // let linearized = linearize_depth(depth, compositor.camera_z_near_far_width_height.x, compositor.camera_z_near_far_width_height.y);
    // let color = hsv_to_rgb(vec3<f32>(linearized, 1.0, 1.0));
    // return vec4<f32>(color, 1.0);
}