#define_import_path bevy_pbr::visbuffer_io

// todo: fix locations

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(2) uv_b: vec2<f32>,
#endif

#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif // MORPH_TARGETS
}

struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

#ifdef VERTEX_UVS_A
    @location(0) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(1) uv_b: vec2<f32>,
#endif

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    @location(0) unclipped_depth: f32,
#endif // UNCLIPPED_DEPTH_ORTHO_EMULATION
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(1) instance_index: u32,
#endif
}

struct FragmentOutput {
#ifdef VISBUFFER_PREPASS
    @location(0) visbuffer: u32,
#endif

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    @builtin(frag_depth) frag_depth: f32,
#endif // UNCLIPPED_DEPTH_ORTHO_EMULATION
}
