struct CameraUniform {
    view_projection: mat4x4<f32>,
    // xyz: camera position in world space; w: 1.0 when the headlight is on.
    eye_shading: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.world_position = input.position;
    return output;
}

const AMBIENT: f32 = 0.25;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Derivatives must be taken in uniform control flow, so compute them
    // unconditionally and blend by the switch afterwards.
    let facing = cross(dpdx(input.world_position), dpdy(input.world_position));
    let facing_length = length(facing);
    let to_eye = normalize(camera.eye_shading.xyz - input.world_position);
    var shade = 1.0;
    if (facing_length > 0.0) {
        shade = AMBIENT + (1.0 - AMBIENT) * abs(dot(facing / facing_length, to_eye));
    }
    let applied = mix(1.0, shade, camera.eye_shading.w);
    return vec4<f32>(input.color.rgb * applied, input.color.a);
}
