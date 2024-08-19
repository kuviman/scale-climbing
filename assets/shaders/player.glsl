varying vec2 v_pos;
varying vec2 v_world_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
uniform vec2 u_pos;
uniform float u_radius;
void main() {
    v_pos = (a_pos * 2.0 - 1.0) * u_radius;
    v_world_pos = u_pos + v_pos;
    vec3 pos = u_projection_matrix * u_view_matrix * vec3(v_world_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform float u_static;
uniform vec2 u_scale_origin;
uniform float u_radius;
void main() {
    if (length(v_pos) > u_radius) {
        discard;
    }
    if (length(v_pos) > u_radius - 0.03) {
        gl_FragColor = vec4(0.2, 0.2, 0.2, 1.0);
        return;
    }
    vec2 from_origin = v_world_pos - u_scale_origin;
    vec3 normal_color = vec3(1.0, 1.0, 1.0);
    vec3 static_color = vec3(0.8, 0.8, 1.0);
    vec3 color = normal_color * (1.0 - u_static) + static_color * u_static;
    if (fract(length(from_origin) * 10.0) < 0.5) {
        color = color * 0.8;
    }
    gl_FragColor = vec4(color, 1.0);
}
#endif