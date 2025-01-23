#import bevy_pbr::{
    pbr_bindings,
    pbr_types,
    prepass_bindings,
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
    visbuffer_io::{Vertex, VertexOutput, FragmentOutput},
    visbuffer_utils::encode_visbuffer,
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[vertex.instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph::morph(vertex_index, morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph::morph(vertex_index, morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph::morph(vertex_index, morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}

// Returns the morphed position of the given vertex from the previous frame.
//
// This function is used for motion vector calculation, and, as such, it doesn't
// bother morphing the normals and tangents.
fn morph_prev_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = morph::prev_weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph::morph(vertex.index, morph::position_offset, i);
        // Don't bother morphing normals and tangents; we don't need them for
        // motion vector calculation.
    }
    return vertex;
}
#endif  // MORPH_TARGETS

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else // SKINNED
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    var world_from_local = mesh_world_from_local;
#endif // SKINNED

    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(world_position.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
#endif // UNCLIPPED_DEPTH_ORTHO_EMULATION

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif // VERTEX_UVS_A

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif // VERTEX_UVS_B

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = vertex_no_morph.instance_index;
#endif

    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(primitive_index) triangle_index: u32) -> FragmentOutput {
    alpha_discard(in);

    var out: FragmentOutput;

    out.visbuffer = encode_visbuffer(in.instance_index, triangle_index);

    return out;
}

// todo: this should really be deduped somehow

// Cutoff used for the premultiplied alpha modes BLEND, ADD, and ALPHA_TO_COVERAGE.
const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn alpha_discard(in: VertexOutput) {

#ifdef MAY_DISCARD
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    var output_color: vec4<f32> = pbr_bindings::material[slot].base_color;
#else   // BINDLESS
    var output_color: vec4<f32> = pbr_bindings::material.base_color;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv_b;
#else   // STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv;
#endif  // STANDARD_MATERIAL_BASE_COLOR_UV_B

#ifdef BINDLESS
    let uv_transform = pbr_bindings::material[slot].uv_transform;
    let flags = pbr_bindings::material[slot].flags;
#else   // BINDLESS
    let uv_transform = pbr_bindings::material.uv_transform;
    let flags = pbr_bindings::material.flags;
#endif  // BINDLESS

    uv = (uv_transform * vec3(uv, 1.0)).xy;
    if (flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        output_color = output_color * textureSampleBias(
#ifdef BINDLESS
            pbr_bindings::base_color_texture[slot],
            pbr_bindings::base_color_sampler[slot],
#else   // BINDLESS
            pbr_bindings::base_color_texture,
            pbr_bindings::base_color_sampler,
#endif  // BINDLESS
            uv,
            view.mip_bias
        );
    }
#endif // VERTEX_UVS

    let alpha_mode = flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
#ifdef BINDLESS
        let alpha_cutoff = pbr_bindings::material[slot].alpha_cutoff;
#else   // BINDLESS
        let alpha_cutoff = pbr_bindings::material.alpha_cutoff;
#endif  // BINDLESS
        if output_color.a < alpha_cutoff {
            discard;
        }
    } else if (alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE) {
        if output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
            discard;
        }
    } else if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        if all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
            discard;
        }
    }

#endif // MAY_DISCARD
}
