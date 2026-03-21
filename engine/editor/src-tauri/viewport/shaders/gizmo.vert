#version 450

layout(location = 0) in vec3 inLocalPos;   // vertex position in gizmo-local space

layout(push_constant) uniform PushConstants {
    mat4  viewProj;
    vec3  gizmoOriginWorld;  // entity world position
    float _pad0;
    vec4  color;             // rgba
    float scale;             // dist * 0.15
    vec3  _pad1;
} pc;

void main() {
    // camera_pos not passed — scale is pre-computed CPU-side from camera distance
    vec3 worldPos = pc.gizmoOriginWorld + inLocalPos * pc.scale;
    gl_Position = pc.viewProj * vec4(worldPos, 1.0);
}
