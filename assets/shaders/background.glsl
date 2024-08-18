varying vec2 v_world_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
void main() {
    vec2 pos = a_pos * 2.0 - 1.0;
    v_world_pos = (inverse(u_projection_matrix * u_view_matrix) * vec3(pos, 1.0)).xy;
    gl_Position = vec4(pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
#include <noise>

float hsla_helper(float h, float s, float l, float n) {
    float alpha = s * min(l, 1.0 - l);
    float k = n + h * 12.0;
    k = k - floor(k / 12.0) * 12.0;
    return l - alpha * max(-1.0, min(min(k - 3.0, 9.0 - k), 1.0));
}

vec4 hsla(float h, float s, float l, float a) {
    return vec4(
        hsla_helper(h, s, l, 0.0),
        hsla_helper(h, s, l, 8.0),
        hsla_helper(h, s, l, 4.0),
        a);
}

uniform float u_time;
void main() {
    float x = snoise(vec3(v_world_pos / 3.0, u_time / 5.0)) * 0.1 + 0.6;
    gl_FragColor = hsla(x, 0.5, 0.1, 1.0);
}
#endif