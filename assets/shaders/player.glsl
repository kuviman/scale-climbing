varying vec2 v_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
uniform vec2 u_pos;
uniform float u_radius;
void main() {
    v_uv = a_pos * 2.0 - 1.0;
    vec3 pos = u_projection_matrix * u_view_matrix * vec3(u_pos + v_uv * u_radius, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
void main() {
    if (length(v_uv) > 1.0) {
        discard;
    }
    gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
}
#endif