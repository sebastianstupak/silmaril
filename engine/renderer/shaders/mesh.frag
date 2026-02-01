#version 450

// Inputs from vertex shader
layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragUV;
layout(location = 2) in vec3 fragWorldPos;

// Output color
layout(location = 0) out vec4 outColor;

void main() {
    // Simple lighting: directional light from above
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    vec3 normal = normalize(fragNormal);

    // Lambertian diffuse
    float diffuse = max(dot(normal, lightDir), 0.0);

    // Ambient + diffuse
    vec3 ambient = vec3(0.2, 0.2, 0.2);
    vec3 color = vec3(0.8, 0.6, 0.4); // Base color

    vec3 finalColor = ambient + color * diffuse;

    outColor = vec4(finalColor, 1.0);
}
