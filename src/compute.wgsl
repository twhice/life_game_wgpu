
@group(0) @binding(0)
var map_textre : texture_2d<f32>;

@group(0) @binding(1)
var this_map : texture_storage_2d<rgba8unorm, write>;

struct MapSize {
    map_size: vec2i,
    _padding: vec2i,
}

@group(0) @binding(2)
var<uniform> map_size : MapSize;

struct ComputeInput {
    @builtin(global_invocation_id) global_id: vec3<u32>
}

@compute @workgroup_size(16, 16)
fn cs_main(in: ComputeInput) {
    let uv = vec2<i32>(in.global_id.xy);
    let offsets = vec3(-1, 0, 1);

    // if uv.x > map_size.map_size.x || uv.y > map_size.map_size.x {
    //     return;
    // }

    var sum: u32 = u32(0);
    sum += is_life(uv + offsets.xx);
    sum += is_life(uv + offsets.yx);
    sum += is_life(uv + offsets.zx);
    sum += is_life(uv + offsets.xy);
    sum += is_life(uv + offsets.zy);
    sum += is_life(uv + offsets.xz);
    sum += is_life(uv + offsets.yz);
    sum += is_life(uv + offsets.zz);
    let last_state = is_life(uv);
    let life = f32(sum == u32(3) || (sum == u32(2) && bool(last_state)));

    textureStore(this_map, vec2<i32>(uv), vec4(life));
}


fn is_life(location: vec2i) -> u32 {

    let location = clamp(location, vec2<i32>(0, 0), map_size.map_size);

    let life = textureLoad(map_textre, location, 0).r > 0.0;
    return u32(life);
}