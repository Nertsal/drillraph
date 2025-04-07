varying vec2 v_vt;
varying vec4 v_color;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_vt;
attribute vec4 a_color;

void main() {
    v_vt = a_vt;
    v_color = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform vec2 u_offset;
uniform vec2 u_scale;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;

void main() {
    vec2 rpos = (v_vt + 1.0) / 2.0 * u_scale;
    vec3 camera_translation = u_projection_matrix * u_view_matrix * vec3(u_offset, 1.0);
    vec2 translation = camera_translation.xy / camera_translation.z;
    vec4 in_color = texture2D(u_texture, fract(rpos - translation));
    gl_FragColor = in_color * v_color;
}
#endif
