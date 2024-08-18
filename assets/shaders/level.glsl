varying vec2 v_world_pos;
varying vec2 v_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
void main() {
    v_uv = a_pos;
    vec2 pos = a_pos * 2.0 - 1.0;
    v_world_pos = (inverse(u_projection_matrix * u_view_matrix) * vec3(pos, 1.0)).xy;
    gl_Position = vec4(pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_surface_dist;

vec4 premultiply(vec3 rgb, float a) {
    return vec4(rgb * a, a);
}

void main() {
    float a = 1.0 - texture2D(u_surface_dist, v_uv).x;
    gl_FragColor = premultiply(vec3(0.5, 0.5, 1.0), a);
}
#endif