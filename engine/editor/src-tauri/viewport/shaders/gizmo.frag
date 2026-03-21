#version 450

layout(push_constant) uniform PushConstants {
    mat4  viewProj;
    vec3  gizmoOriginWorld;
    float _pad0;
    vec4  color;
    float scale;
    vec3  _pad1;
} pc;

layout(location = 0) out vec4 outColor;

void main() {
    outColor = pc.color;
}
