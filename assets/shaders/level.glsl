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
#include <noise>
uniform sampler2D u_surface_dist;

vec4 premultiply(vec3 rgb, float a) {
    return vec4(rgb * a, a);
}

void main() {
    float d = texture2D(u_surface_dist, v_uv).x * 2.0 - 1.0;
    if (d < -0.5) {
        float t = floor(snoise(v_world_pos) * 0.5 + snoise(v_world_pos / 2.0 + vec2(123.0, 56.0)));
        t = 0.05 * t + 0.25;
        gl_FragColor = vec4(t, t, t, 1.0);
    } else if (d > 0.5) {
        discard;
    } else {
        gl_FragColor = vec4(vec3(0.1), 1.0);
    }
}
#endif