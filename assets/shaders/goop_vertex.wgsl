#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings,
    view_transformations::position_world_to_clip,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{Vertex, VertexOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

#import noisy_bevy::simplex_noise_3d;



@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> noise_frequency: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> extent: f32;



@vertex
fn vertex(in: Vertex) -> VertexOutput {

    var curr_pos = in.position;
    var curr_normal = in.normal;

    const x_vec = vec3f(1.0, 0.0, 0.0);
    const y_vec = vec3f(0.0, 1.0, 0.0);
    var curr_tangent = vec3f(0.0);
    if (length( in.normal - x_vec ) < 0.1 ) {
        curr_tangent = normalize(cross(in.normal, y_vec));
    } else {
        curr_tangent = normalize(cross(in.normal, x_vec));
    }

    var curr_bitangent = cross(curr_normal, curr_tangent);


    const EPSILON: f32 = 0.01;
    let noise_frequencies = array<f32,3>(0.3 * noise_frequency, 0.4 * noise_frequency, 0.9 * noise_frequency);
    let height_scales = array<f32,3>(extent, extent * 0.6, extent * 0.3);
    for (var i = 0; i < 3; i++) {
        let curr_noise_frequency = noise_frequencies[i];
        let height_scale = height_scales[i];

        let noise_value = simplex_noise_3d(curr_pos * curr_noise_frequency) * height_scale;
        let offset = curr_normal * noise_value;
        let displaced_pos = curr_pos + offset;

        let h_tangent = simplex_noise_3d(
            (curr_pos + curr_tangent * EPSILON)
            * curr_noise_frequency
        ) * height_scale;
        let h_bitangent = simplex_noise_3d(
            (curr_pos + curr_bitangent * EPSILON)
            * curr_noise_frequency
        ) * height_scale;

        let offset_tangent = curr_tangent * EPSILON + curr_normal * h_tangent;
        let offset_bitangent = curr_bitangent * EPSILON + curr_normal * h_bitangent;

        let new_tangent = normalize(offset_tangent - offset);
        let new_bitangent = normalize(offset_bitangent - offset);

        let new_normal = normalize(cross(new_tangent, new_bitangent));


        curr_normal = new_normal;
        curr_tangent = new_tangent;
        curr_bitangent = new_bitangent;
        curr_pos = displaced_pos;
    }

    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);
    let local_position = vec4(curr_pos, 1.0);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, local_position);
    out.position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_tangent_local_to_world(world_from_local, vec4f(curr_normal, 1.0), in.instance_index).xyz;

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vec4f(curr_tangent, 1.0), in.instance_index);
#endif
    out.uv = in.uv;
    out.instance_index = in.instance_index;

    return out;
}

