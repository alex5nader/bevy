#define_import_path bevy_pbr::prepass_utils

#import bevy_pbr::mesh_view_bindings as view_bindings

#ifdef DEPTH_PREPASS
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 {
#ifdef MULTISAMPLED
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else // MULTISAMPLED
    return textureLoad(view_bindings::depth_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif // MULTISAMPLED
}
#endif // DEPTH_PREPASS

#ifdef NORMAL_PREPASS
fn prepass_normal(frag_coord: vec4<f32>, sample_index: u32) -> vec3<f32> {
#ifdef MULTISAMPLED
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let normal_sample = textureLoad(view_bindings::normal_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif // MULTISAMPLED
    return normalize(normal_sample.xyz * 2.0 - vec3(1.0));
}
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
fn prepass_motion_vector(frag_coord: vec4<f32>, sample_index: u32) -> vec2<f32> {
#ifdef MULTISAMPLED
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), i32(sample_index));
#else
    let motion_vector_sample = textureLoad(view_bindings::motion_vector_prepass_texture, vec2<i32>(frag_coord.xy), 0);
#endif
    return motion_vector_sample.rg;
}
#endif // MOTION_VECTOR_PREPASS

#ifdef VISBUFFER_PREPASS
fn prepass_visbuffer(frag_coord: vec4<f32>, sample_index: u32) -> u32 {
    let visbuffer_sample = textureLoad(view_bindings::visbuffer_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    return visbuffer_sample.r;
}

fn encode_visbuffer(instance_index: u32, triangle_index: u32) -> u32 {
    return (0xFFFFu & instance_index) | ((0xFFFFu & triangle_index) << 16u);
}

fn decode_visbuffer_instance(visbuffer: u32) -> u32 {
    return 0xFFFFu & visbuffer;
}

fn decode_visbuffer_triangle(visbuffer: u32) -> u32 {
    return visbuffer >> 16u;
}
#endif
