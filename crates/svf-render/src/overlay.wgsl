// Screen-space overlay: solid quads (uv < 0) and glyph quads (coverage from the atlas).
struct Target {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> frame: Target;
@group(0) @binding(1)
var atlas: texture_2d<f32>;
@group(0) @binding(2)
var atlas_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    // Pixels, top-left origin, to clip space.
    let ndc = vec2<f32>(position.x / frame.size.x * 2.0 - 1.0, 1.0 - position.y / frame.size.y * 2.0);
    output.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    output.uv = uv;
    output.colour = colour;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas, atlas_sampler, max(input.uv, vec2<f32>(0.0, 0.0))).r;
    let alpha = select(coverage, 1.0, input.uv.x < 0.0);
    return vec4<f32>(input.colour.rgb, input.colour.a * alpha);
}
