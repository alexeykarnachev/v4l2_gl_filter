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

    float lumin = dot(color, vec3(0.2126, 0.7152, 0.0722));
    color = vec3(floor(lumin + 0.5));

    frag_color = vec4(color, 1.0);
}
