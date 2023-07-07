// 实际上只用两个三角形铺满屏幕
struct VertexInput {
    @location(0) position: vec2f,
    @location(1) texcorrd: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) texcorrd: vec2f,
}

@group(0) @binding(0)
var map_textre : texture_2d<f32>;

@group(0) @binding(1)
var map_sampler : sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4(in.position, 0.0, 1.0);
    out.texcorrd = in.texcorrd;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let color: vec4f = textureSample(map_textre, map_sampler, in.texcorrd);


    return color;
}
