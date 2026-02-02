#version 450

// Vertex input (matches engine_assets::Vertex layout)
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// Vertex output to fragment shader
layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragPosition;

// Push constant for MVP matrix (64 bytes)
layout(push_constant) uniform PushConstants {
    mat4 mvp;
} pc;

void main() {
    // Transform position
    gl_Position = pc.mvp * vec4(inPosition, 1.0);

    // Pass through to fragment shader
    fragNormal = inNormal;
    fragUV = inUV;
    fragPosition = inPosition;
}
