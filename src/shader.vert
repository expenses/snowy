#version 450

layout(location = 0) in vec2 v_point;

layout(location = 1) in vec2 i_center;
layout(location = 2) in vec2 i_dimensions;
layout(location = 3) in float i_rotation;
layout(location = 4) in vec2 i_uv_top_left;
layout(location = 5) in vec4 i_overlay;

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_overlay;

layout(set = 0, binding = 2) uniform Uniforms {
    vec2 window_size;
};

void main() {
    //vec2 pos = i_center * vec2(1.0, -1.0);

    mat2 rotation = mat2(
        cos(i_rotation), -sin(i_rotation),
        sin(i_rotation),  cos(i_rotation)
    );

    vec2 tiles_pos = i_center + (rotation * (i_dimensions * v_point));

    gl_Position = vec4(tiles_pos / window_size, 0.0, 1.0);

    vec2 uv_dimensions = vec2(1.0 / 4.0, 1.0 / 4.0);

    vec2 uv_offset = i_uv_top_left + uv_dimensions * 0.5;

    out_overlay = i_overlay;
    out_uv = uv_offset + uv_dimensions * vec2(1.0, -1.0) * v_point * 0.5;
}
