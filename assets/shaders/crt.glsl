// adapted from <https://www.shadertoy.com/view/WsVSzV>

varying vec2 v_vt;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_vt;

void main() {
    v_vt = a_vt;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;

const float warp = 0.25; // simulate curvature of CRT monitor
const float scan = 0.5; // simulate darkness between scanlines

void main() {
    // squared distance from center
    vec2 uv = v_vt;
    vec2 dc = abs(0.5 - uv);
    dc *= dc;
    
    // warp the fragment coordinates
    uv.x -= 0.5; uv.x *= 1.0 + (dc.y * (0.3 * warp)); uv.x += 0.5;
    uv.y -= 0.5; uv.y *= 1.0 + (dc.x * (0.4 * warp)); uv.y += 0.5;

    // sample inside boundaries, otherwise set to black
    if (uv.y > 1.0 || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        // determine if we are drawing in a scanline
        float apply = abs(sin(v_vt.y) * 0.5 * scan);
        // sample the texture
        gl_FragColor = vec4(mix(texture2D(u_texture, uv).rgb, vec3(0.0), apply), 1.0);
    }
}
#endif
