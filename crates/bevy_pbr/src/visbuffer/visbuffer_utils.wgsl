#define_import_path bevy_pbr::visbuffer_utils

#import bevy_pbr::mesh_view_bindings as view_bindings

#ifdef VISBUFFER_PREPASS
fn encode_visbuffer(instance_index: u32, triangle_index: u32) -> u32 {
    return (0xFFFFu & ~instance_index) | ((0xFFFFu & triangle_index) << 16u);
}
#endif  // VISBUFFER_PREPASS

#ifdef VISBUFFER_RESOLVE
fn decode_visbuffer_instance(visbuffer: u32) -> u32 {
    return ~(0xFFFFu & visbuffer);
}

fn decode_visbuffer_triangle(visbuffer: u32) -> u32 {
    return visbuffer >> 16u;
}
#endif  // VISBUFFER_RESOLVE

fn prepass_visbuffer(frag_coord: vec4<f32>, sample_index: u32) -> u32 {
    let visbuffer_sample = textureLoad(view_bindings::visbuffer_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return visbuffer_sample.r;
}

