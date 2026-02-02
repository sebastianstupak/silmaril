#version 450

// Input from vertex shader
layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragUV;
layout(location = 2) in vec3 fragPosition;

// Output color
layout(location = 0) out vec4 outColor;

void main() {
    // Simple diffuse lighting (directional light from above)
    vec3 lightDir = normalize(vec3(0.0, -1.0, -0.5));
    vec3 normal = normalize(fragNormal);
    float diffuse = max(dot(normal, -lightDir), 0.0);

    // Base color (white) with diffuse lighting
    vec3 baseColor = vec3(1.0, 1.0, 1.0);
    vec3 ambient = baseColor * 0.3;
    vec3 lit = ambient + baseColor * diffuse * 0.7;

    outColor = vec4(lit, 1.0);
}
