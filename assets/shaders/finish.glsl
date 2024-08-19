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
#include <noise>

uniform float u_time;

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

vec4 premultiply(vec3 rgb, float a) {
    return vec4(rgb * a, a);
}

void main() {
    if (length(v_uv) > 1.0) {
        discard;
    }
    float a = float(fract(atan(v_uv.y, v_uv.x) / 3.14 * 5.0 + u_time) > length(v_uv));
    a = max(a, 0.7 - length(v_uv));
    gl_FragColor = premultiply(hsla(snoise(vec3(v_uv, u_time)) * 0.2 + u_time * 0.2, 1.0, 0.4, 1.0).rgb, a);
}
#endif