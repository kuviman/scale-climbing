#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
uniform mat3 u_model_matrix;
void main() {
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos * 2.0 - 1.0, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
void main() {
    gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
}
#endif