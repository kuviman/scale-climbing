#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
void main() {
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
void main() {
    gl_FragColor = vec4(0.3, 0.3, 0.3, 1.0);
}
#endif