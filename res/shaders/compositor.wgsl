
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct CompositorUniform {
    @location(0) tint: vec4<f32>,
}

@group(0) @binding(0)
var t_color_attachment: texture_2d<f32>;

@group(0) @binding(1)
var s_color_attachment: sampler;

@group(1) @binding(0)
var<uniform> compositor: CompositorUniform;


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
    // return vec4<f32>(in.tex_coord, 0.0, 1.0) * compositor.tint;

    return textureSample(t_color_attachment, s_color_attachment, in.tex_coord);
    // let tinted = color;
    // return vec4<f32>(tinted.rgb, 1.0);
}