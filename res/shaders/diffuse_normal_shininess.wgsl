//
//  Uniforms
//

struct Material {
    ambient: vec4<f32>;
    diffuse: vec4<f32>;
    specular: vec4<f32>;
    shininess: f32;
};

struct CameraUniform {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

struct Light {
    position: vec3<f32>;
    color: vec3<f32>;
};

[[group(0), binding(0)]]
var<uniform> material: Material;

[[group(0), binding(1)]]
var t_diffuse: texture_2d<f32>;

[[group(0), binding(2)]]
var s_diffuse: sampler;

[[group(0), binding(3)]]
var t_normal: texture_2d<f32>;

[[group(0), binding(4)]]
var s_normal: sampler;

[[group(0), binding(5)]]
var t_shininess: texture_2d<f32>;

[[group(0), binding(6)]]
var s_shininess: sampler;

[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

[[group(2), binding(0)]]
var<uniform> light: Light;

//
//  Model
//

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(3)]] tangent: vec3<f32>;
    [[location(4)]] bitangent: vec3<f32>;
};

struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;

    [[location(9)]] normal_matrix_1: vec3<f32>;
    [[location(10)]] normal_matrix_2: vec3<f32>;
    [[location(11)]] normal_matrix_3: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] tangent_position: vec3<f32>;
    [[location(2)]] tangent_light_position: vec3<f32>;
    [[location(3)]] tangent_view_position: vec3<f32>;
};

//
// Vertex
//

[[stage(vertex)]]
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_1,
        instance.normal_matrix_2,
        instance.normal_matrix_3,
    );

    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal
    ));

    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
    out.tangent_light_position = tangent_matrix * light.position;

    return out;
}

//
// Fragment
//

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color:vec4<f32> = material.diffuse * textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal:vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    let object_shininess:vec4<f32> = textureSample(t_shininess, s_shininess, in.tex_coords);

    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = normalize(in.tangent_light_position - in.tangent_position);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);

    let ambient_strength = 0.1 * material.ambient.rgb;
    let ambient_color = light.color * ambient_strength;

    let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), object_shininess.g * material.shininess);
    let specular_color = object_shininess.r * specular_strength * light.color * material.specular.rgb;

    let result = (ambient_color + diffuse_color) * object_color.rgb + specular_color;
    return vec4<f32>(result, object_color.a);
}