#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph,
    morph::{morph_position, morph_normal, morph_tangent},
    prepass_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
}

#import bevy_render::globals::Globals

@group(0) @binding(1) var<uniform> globals: Globals;

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let weight_tensor_key = morph::get_weight_tensor_key(instance_index);
    vertex.position += morph_position(vertex_in.index, weight_tensor_key);
#ifdef VERTEX_NORMALS
    vertex.normal += morph_normal(vertex_in.index, weight_tensor_key);
#endif
#ifdef VERTEX_TANGENTS
    vertex.tangent += vec4<f32>(morph_tangent(vertex_in.index, weight_tensor_key), 0.0);
#endif
    return vertex;
}

#ifdef HAS_PREVIOUS_MORPH
fn morph_prev_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let weight_count = morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = morph::prev_weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
    }
    return vertex;
}
#endif
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif
#endif

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    
    // --- GRASS WIND SWAY LOGIC ---
#ifdef VERTEX_UVS_A
    let sway_factor = 1.0 - vertex.uv.y;
#else
    let sway_factor = 0.0;
#endif
    let time = globals.time;
    let wave = sin(out.world_position.x * 0.15 + time * 1.2) * 0.6 + cos(out.world_position.z * 0.1 + time * 1.0) * 0.4;
    
    out.world_position.x += sway_factor * sway_factor * wave * 0.18;
    out.world_position.z += sway_factor * sway_factor * wave * 0.12;
    // -----------------------------
    
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef MOTION_VECTOR_PREPASS
    // Take morph targets into account.
#ifdef MORPH_TARGETS
#ifdef HAS_PREVIOUS_MORPH
    let prev_vertex = morph_prev_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    let prev_vertex = vertex_no_morph;
#endif
#else
    let prev_vertex = vertex_no_morph;
#endif

    // Take skinning into account.
#ifdef SKINNED
#ifdef HAS_PREVIOUS_SKIN
    let prev_model = skinning::skin_prev_model(
        prev_vertex.joint_indices,
        prev_vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_no_morph.instance_index);
#endif
#else
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_no_morph.instance_index);
#endif

    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        prev_model,
        vec4<f32>(prev_vertex.position, 1.0)
    );
#endif // MOTION_VECTOR_PREPASS

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}
