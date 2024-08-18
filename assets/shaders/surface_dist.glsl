varying vec2 v_dist;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_dist;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
void main() {
    v_dist = a_dist;
    vec3 pos = u_projection_matrix * u_view_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform float u_max_distance;
void main() {
    float d = length(v_dist) / u_max_distance;
    gl_FragColor = vec4(d, d, d, 1.0);
}
#endif