//
//  Uniforms
//

struct Material {
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
    glossiness: f32,
};

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    proj_inverse: mat4x4<f32>,
    view_inverse: mat4x4<f32>,
};

struct Light {
    position: vec3<f32>,
    direction: vec3<f32>,
    ambient: vec3<f32>,
    color: vec3<f32>,

    // x: constant, y: linear, z: exponential, w: dot spot breadth
    attenuation: vec4<f32>,

    // 0: Ambient
    // 1: Point
    // 2: Spot
    // 3: Directional
    light_type: i32,

};

@group(0) @binding(0)
var<uniform> material: Material;

@group(0) @binding(1)
var environment_map_texture: texture_cube<f32>;

@group(0) @binding(2)
var environment_map_sampler: sampler;

@group(0) @binding(3)
var diffuse_texture: texture_2d<f32>;

@group(0) @binding(4)
var diffuse_sampler: sampler;

@group(0) @binding(5)
var normal_texture: texture_2d<f32>;

@group(0) @binding(6)
var normal_sampler: sampler;

@group(0) @binding(7)
var glossiness_texture: texture_2d<f32>;

@group(0) @binding(8)
var glossiness_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> light: Light;

//
//  Model
//

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    @location(9) normal_matrix_1: vec3<f32>,
    @location(10) normal_matrix_2: vec3<f32>,
    @location(11) normal_matrix_3: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec3<f32>,
    @location(3) world_bitangent: vec3<f32>,
    @location(4) tex_coords: vec2<f32>,
    @location(5) tangent_position: vec3<f32>,
    @location(6) tangent_view_position: vec3<f32>,
    @location(7) tangent_light_position: vec3<f32>,
    @location(8) tangent_light_dir: vec3<f32>,
};

//
//  Util
//

// returns [0,1] for where v lands in range [a,b]. Result is unclamped.
fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
    return (v - a) / (b - a);
}

// Returns the light dir depending on light type. Note, this is direction TO the light.
fn fs_get_light_dir(in: VertexOutput) -> vec3<f32> {
    if (light.light_type == 1 || light.light_type == 2) {
        // point or spot
        return normalize(in.tangent_light_position - in.tangent_position);
    } else {
        // directional
        return normalize(in.tangent_light_dir);
    }
}

fn fs_compute_light_attenuation(in: VertexOutput) -> f32 {
    let light_distance = length(light.position - in.world_position.xyz);
    var light_attenuation = 1.0 / (light.attenuation.x + (light.attenuation.y * light_distance) + (light.attenuation.z * light_distance * light_distance));

    if (light.light_type == 2) {
        // spot light
        let to_light = normalize(in.world_position.xyz - light.position);
        let d = clamp(dot(to_light, light.direction), 0.0, 1.0);
        let spot = inverse_lerp(light.attenuation.w, 1.0, d);
        light_attenuation = light_attenuation * spot;
    }

    return light_attenuation;
}

//
// Vertex
//

@vertex
fn vs_main_ambient(model: VertexInput, instance: InstanceInput) -> VertexOutput {
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

    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position;
    out.tex_coords = model.tex_coords;
    out.world_normal = normal_matrix * model.normal;
    out.world_tangent = normal_matrix * model.tangent;
    out.world_bitangent = normal_matrix * model.bitangent;
    return out;
}

@vertex
fn vs_main_lit(model: VertexInput, instance: InstanceInput) -> VertexOutput {
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
    out.world_position = world_position;
    out.tex_coords = model.tex_coords;
    out.world_normal = world_normal;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    out.tangent_light_dir = tangent_matrix * light.direction;

    return out;
}

//
// Fragment Ambient
//

@fragment
fn fs_main_ambient_untextured(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = material.diffuse;
    let object_normal = in.world_normal;
    let reflection_dir = reflect(normalize(in.world_position.xyz - camera.view_pos.xyz), object_normal);
    let environment_color = textureSample(environment_map_texture, environment_map_sampler, in.world_normal).rgb;
    let environment_reflection = material.specular.rgb * textureSample(environment_map_texture, environment_map_sampler, reflection_dir).rgb;
    let ambient_color = (environment_color.rgb * material.ambient.rgb * object_color.rgb) + (light.ambient * object_color.rgb);

    return vec4<f32>(environment_reflection + ambient_color, object_color.a);
}

@fragment
fn fs_main_ambient_diffuse(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let object_normal = in.world_normal;
    let reflection_dir = reflect(normalize(in.world_position.xyz - camera.view_pos.xyz), object_normal);
    let environment_color = textureSample(environment_map_texture, environment_map_sampler, in.world_normal).rgb;
    let environment_reflection = material.specular.rgb * textureSample(environment_map_texture, environment_map_sampler, reflection_dir).rgb;
    let ambient_color = (environment_color.rgb * material.ambient.rgb * object_color.rgb) + (light.ambient * object_color.rgb);

    return vec4<f32>(environment_reflection + ambient_color, object_color.a);
}

@fragment
fn fs_main_ambient_diffuse_normal(in: VertexOutput) -> @location(0) vec4<f32> {
    let tangent_to_world = mat3x3<f32>(
        in.world_tangent,
        in.world_bitangent,
        in.world_normal
    );

    let object_color = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let object_normal = tangent_to_world * (textureSample(normal_texture, normal_sampler, in.tex_coords).xyz * 2.0 - 1.0);
    let reflection_dir = reflect(normalize(in.world_position.xyz - camera.view_pos.xyz), object_normal);
    let environment_color = textureSample(environment_map_texture, environment_map_sampler, object_normal);
    let environment_reflection = material.specular.rgb * textureSample(environment_map_texture, environment_map_sampler, reflection_dir).rgb;
    let ambient_color = (environment_color.rgb * material.ambient.rgb * object_color.rgb) + (light.ambient * object_color.rgb);
    return vec4<f32>(ambient_color, object_color.a);
}

@fragment
fn fs_main_ambient_diffuse_normal_glossiness(in: VertexOutput) -> @location(0) vec4<f32> {
    let tangent_to_world = mat3x3<f32>(
        in.world_tangent,
        in.world_bitangent,
        in.world_normal
    );

    let object_color = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let object_normal = tangent_to_world * (textureSample(normal_texture, normal_sampler, in.tex_coords).xyz * 2.0 - 1.0);
    let object_glossiness = material.specular.rgb * textureSample(glossiness_texture, glossiness_sampler, in.tex_coords).r;
    let reflection_dir = reflect(normalize(in.world_position.xyz - camera.view_pos.xyz), object_normal);
    let environment_color = textureSample(environment_map_texture, environment_map_sampler, object_normal);
    let environment_reflection = object_glossiness * textureSample(environment_map_texture, environment_map_sampler, reflection_dir).rgb;
    let ambient_color = (environment_color.rgb * material.ambient.rgb * object_color.rgb) + (light.ambient * object_color.rgb);
    return vec4<f32>(ambient_color, object_color.a);
}


//
//  Fragment Lit
//

@fragment
fn fs_main_lit_diffuse_normal_glossiness(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color:vec4<f32> = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let object_normal:vec4<f32> = textureSample(normal_texture, normal_sampler, in.tex_coords);
    let object_glossiness:vec4<f32> = textureSample(glossiness_texture, glossiness_sampler, in.tex_coords);

    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = fs_get_light_dir(in);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);
    let light_attenuation = fs_compute_light_attenuation(in);

    let diffuse_strength = light_attenuation * max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = light_attenuation * pow(max(dot(tangent_normal, half_dir), 0.0), object_glossiness.g * material.glossiness);
    let specular_color = object_glossiness.r * specular_strength * light.color * material.specular.rgb;

    let result = (diffuse_color * object_color.rgb) + specular_color;
    return vec4<f32>(result, object_color.a);
}

@fragment
fn fs_main_lit_diffuse_normal(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color:vec4<f32> = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let object_normal:vec4<f32> = textureSample(normal_texture, normal_sampler, in.tex_coords);

    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = fs_get_light_dir(in);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);
    let light_attenuation = fs_compute_light_attenuation(in);

    let diffuse_strength = light_attenuation * max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = light_attenuation * pow(max(dot(tangent_normal, half_dir), 0.0), material.glossiness);
    let specular_color = material.specular.rgb * specular_strength * light.color;

    let result = (diffuse_color * object_color.rgb) + specular_color;
    return vec4<f32>(result, object_color.a);
}

@fragment
fn fs_main_lit_diffuse(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color:vec4<f32> = material.diffuse * textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);

    let tangent_normal = vec3<f32>(0.0, 0.0, 1.0);
    let light_dir = fs_get_light_dir(in);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);
    let light_attenuation = fs_compute_light_attenuation(in);

    let diffuse_strength = light_attenuation * max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = light_attenuation * pow(max(dot(tangent_normal, half_dir), 0.0), material.glossiness);
    let specular_color = material.specular.rgb * specular_strength * light.color;

    let result = (diffuse_color * object_color.rgb) + specular_color;
    return vec4<f32>(result, object_color.a);
}

@fragment
fn fs_main_lit_untextured(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color:vec4<f32> = material.diffuse;

    let tangent_normal = vec3<f32>(0.0, 0.0, 1.0);
    let light_dir = fs_get_light_dir(in);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);
    let light_attenuation = fs_compute_light_attenuation(in);

    let diffuse_strength = light_attenuation * max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = light_attenuation * pow(max(dot(tangent_normal, half_dir), 0.0), material.glossiness);
    let specular_color = material.specular.rgb * specular_strength * light.color;

    let result = (diffuse_color * object_color.rgb) + specular_color;
    return vec4<f32>(result, object_color.a);
}