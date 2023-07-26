#version 460 core

in vec2 vs_texcoord;

out vec4 frag_color;

uniform sampler2D u_tex;

const vec2 POISSON_DISK[87] = vec2[87](vec2(-0.488690, 0.046349), vec2(0.496064, 0.018367), vec2(-0.027347, -0.461505), vec2(-0.090074, 0.490283), vec2(0.294474, 0.366950), vec2(0.305608, -0.360041), vec2(-0.346198, -0.357278), vec2(-0.308924, 0.353038), vec2(-0.437547, -0.177748), vec2(0.446996, -0.129850), vec2(0.117621, -0.444649), vec2(0.171424, 0.418258), vec2(-0.227789, -0.410446), vec2(0.210264, -0.422608), vec2(-0.414136, -0.268376), vec2(0.368202, 0.316549), vec2(-0.480689, 0.127069), vec2(0.481128, -0.056358), vec2(-0.458004, -0.063002), vec2(0.409361, 0.201972), vec2(-0.176597, 0.424044), vec2(-0.095380, -0.441734), vec2(0.326086, -0.280594), vec2(-0.411327, 0.184757), vec2(-0.291534, -0.300406), vec2(0.400901, -0.002308), vec2(0.020255, 0.445511), vec2(0.302251, 0.275637), vec2(0.387805, -0.223370), vec2(-0.378395, 0.062614), vec2(0.405052, 0.101681), vec2(-0.010340, -0.355322), vec2(-0.034931, 0.383699), vec2(-0.318953, -0.225899), vec2(0.349283, -0.140001), vec2(-0.253974, 0.299183), vec2(0.188226, 0.342914), vec2(0.212083, -0.294545), vec2(-0.188320, -0.308466), vec2(-0.373708, -0.070538), vec2(0.114322, -0.356677), vec2(-0.154401, 0.348207), vec2(-0.321713, 0.260043), vec2(-0.086797, -0.349277), vec2(-0.360294, -0.144808), vec2(-0.323996, 0.188199), vec2(0.277830, -0.204128), vec2(0.087828, 0.351992), vec2(-0.215777, -0.234955), vec2(0.291437, 0.171860), vec2(0.027249, -0.255925), vec2(-0.316361, -0.013941), vec2(0.346679, -0.066942), vec2(-0.103280, -0.273636), vec2(-0.017802, 0.310973), vec2(-0.280809, -0.120043), vec2(-0.282912, 0.117500), vec2(0.267574, -0.036973), vec2(-0.034965, -0.223502), vec2(0.109677, 0.256372), vec2(-0.204519, -0.116846), vec2(0.144105, -0.181736), vec2(-0.140560, 0.215101), vec2(0.271573, 0.102406), vec2(0.220437, 0.203459), vec2(-0.242979, -0.027494), vec2(-0.050135, 0.239871), vec2(-0.152652, -0.193125), vec2(-0.220532, 0.179600), vec2(0.216867, -0.096770), vec2(-0.164884, 0.122109), vec2(0.251078, 0.034090), vec2(0.016515, -0.175206), vec2(0.042304, 0.216117), vec2(-0.133933, -0.060601), vec2(0.184659, 0.135680), vec2(-0.161273, 0.024207), vec2(-0.056532, -0.154410), vec2(-0.082706, 0.083129), vec2(0.081409, -0.088060), vec2(0.115078, 0.156566), vec2(0.133209, 0.061211), vec2(0.002618, -0.101328), vec2(0.132926, -0.013988), vec2(-0.027172, -0.017586), vec2(0.022969, 0.116469), vec2(0.036262, 0.015085));

float rand(vec2 seed) {
    return fract(sin(dot(seed.xy, vec2(12.9898, 78.233))) * 43758.5453);
}

vec2 sample_poisson_disc(vec2 seed, int idx) {
    int offset_ = int(rand(seed) * 87.0);
    idx = (offset_ + idx) % 87;
    return POISSON_DISK[idx];
}

vec3 sample_texture(sampler2D tex, vec2 uv, float n_samples, float radius) {
    ivec2 size = textureSize(tex, 0);
    vec2 uv_step = 1.0 / vec2(size);
    vec3 color = vec3(0.0);
    float n = 0.0;
    for(int i = 0; i < int(n_samples); ++i) {
        vec2 disc = sample_poisson_disc(uv, i);
        vec2 uv_ = uv + radius * uv_step * disc;
        if(uv_.x >= 0.0 && uv_.x <= 1.0 && uv_.y >= 0.0 && uv_.y <= 1.0) {
            color += texture(tex, uv_).rgb;
            n += 1.0;
        }
    }
    color /= n;
    return color;
}

vec3 rgb2hsv(vec3 c) {
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

float quantize(float x, float n_levels) {
    return round(x * n_levels) / n_levels;
}

vec3 quantize_vec3(vec3 v, float n_levels) {
    return round(v * n_levels) / n_levels;
}

vec3 quantize_color(vec3 color, float n_levels) {
    if(n_levels == 0.0) {
        return color;
    }
    vec3 hsv = rgb2hsv(color);
    hsv.x = quantize(hsv.x, n_levels);
    hsv.z = quantize(hsv.z, n_levels);
    return hsv2rgb(hsv);
}

void main() {
    vec2 uv = vec2(vs_texcoord.x, 1.0 - vs_texcoord.y);
    vec3 color = sample_texture(u_tex, uv, 87, 16).rgb;
    color = quantize_color(color, 4);
    frag_color = vec4(color, 1.0);
}
