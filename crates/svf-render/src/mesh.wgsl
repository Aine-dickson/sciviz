struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(-0.35, 0.85, 0.4));
    let normal = vec3<f32>(0.0, 1.0, 0.0);
    let diffuse = max(dot(normal, light), 0.0);
    let illumination = 0.35 + 0.65 * diffuse;
    return vec4<f32>(input.color * illumination, 1.0);
}
