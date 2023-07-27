#version 460 core

#define PI 3.141592653589793238462643383279502884197

in vec2 vs_texcoord;

out vec4 frag_color;

uniform float u_time;
uniform sampler2D u_video_tex;

void main() {
    vec2 uv = vec2(vs_texcoord.x, 1.0 - vs_texcoord.y);

    if (uv.x >= 0.4 && uv.x <= 0.6 && uv.y >= 0.4 && uv.y <= 0.6) {
        uv -= 0.5;
        float a = 0.1 * PI * u_time / 1000.0;
        float c = cos(a);
        float s = sin(a);
        float x = uv.x * c - uv.y * s;
        float y = uv.x * s + uv.y * c;
        uv = vec2(x, y);
        uv += 0.5;
    }

    vec3 color = texture(u_video_tex, uv).rgb;
    color.r = color.r * 0.5 * (sin(4.0 * u_time / 1000.0) + 1.0);
    frag_color = vec4(color, 1.0);
}
