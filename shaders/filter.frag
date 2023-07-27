#version 460 core

#define PI 3.141592653589793238462643383279502884197

in vec2 vs_texcoord;

out vec4 frag_color;

uniform float u_time;
uniform float u_width;
uniform float u_height;
uniform sampler2D u_video_tex;

void main() {
    vec2 uv = vec2(vs_texcoord.x, 1.0 - vs_texcoord.y);
    vec3 color = texture(u_video_tex, uv).rgb;
    color *= vec3(uv, 0.5 * (sin(10.0 * u_time / 1000.0) + 1.0));

    frag_color = vec4(color, 1.0);
}
