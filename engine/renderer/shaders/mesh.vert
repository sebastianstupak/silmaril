#version 450

// Vertex attributes
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// Push constants for MVP matrix
layout(push_constant) uniform PushConstants {
    mat4 mvp;
} pc;

// Outputs to fragment shader
layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragWorldPos;

void main() {
    gl_Position = pc.mvp * vec4(inPosition, 1.0);

    // Pass through to fragment shader
    fragNormal = inNormal;
    fragUV = inUV;
    fragWorldPos = inPosition;
}
