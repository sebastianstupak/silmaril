#version 450

// ── Vertex attributes ────────────────────────────────────────────────────────
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// ── Fragment shader outputs ──────────────────────────────────────────────────
layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragPosition;

// ── Push constants (VP matrix only — 64 bytes) ──────────────────────────────
layout(push_constant) uniform PushConstants {
    mat4 vp;
} pc;

// ── Storage buffer: one MeshUniform per renderable entity ───────────────────
struct MeshUniform {
    mat4 world_from_local;
    mat4 local_from_world_transpose;
};

layout(set = 0, binding = 0) readonly buffer MeshUniforms {
    MeshUniform entries[];
} mesh_data;

// ── Main ────────────────────────────────────────────────────────────────────
void main() {
    MeshUniform mu = mesh_data.entries[gl_InstanceIndex];

    gl_Position  = pc.vp * mu.world_from_local * vec4(inPosition, 1.0);
    fragNormal   = mat3(mu.local_from_world_transpose) * inNormal;
    fragUV       = inUV;
    fragPosition = vec3(mu.world_from_local * vec4(inPosition, 1.0));
}
