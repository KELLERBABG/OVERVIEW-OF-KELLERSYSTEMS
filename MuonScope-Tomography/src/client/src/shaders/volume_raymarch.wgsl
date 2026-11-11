// =============================================================================
// WebGPU WGSL Volumetric Raymarching Shader for Muon Tomography 3D Density Grid
// =============================================================================

struct Uniforms {
    viewProjectionMatrix : mat4x4<f32>,
    cameraPosition       : vec3<f32>,
    stepSize             : f32,
    densityThreshold     : f32,
};

@group(0) @binding(0) var<uniform> uniforms : Uniforms;
@group(0) @binding(1) var volumeTexture : texture_3d<f32>;
@group(0) @binding(2) var volumeSampler : sampler;

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0) worldPos       : vec3<f32>,
};

@vertex
fn vs_main(@location(0) position : vec3<f32>) -> VertexOutput {
    var output : VertexOutput;
    output.position = uniforms.viewProjectionMatrix * vec4<f32>(position, 1.0);
    output.worldPos = position;
    return output;
}

@fragment
fn fs_main(input : VertexOutput) -> @location(0) vec4<f32> {
    let rayDir = normalize(input.worldPos - uniforms.cameraPosition);
    var samplePos = input.worldPos;
    var accumulatedColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // Front-to-back raymarching loop
    for (var i = 0; i < 128; i++) {
        // Map world coordinates [-0.5, 0.5] to texture UVW [0.0, 1.0]
        let uvw = samplePos + vec3<f32>(0.5, 0.5, 0.5);

        if (any(uvw < vec3<f32>(0.0)) || any(uvw > vec3<f32>(1.0))) {
            break;
        }

        let density = textureSample(volumeTexture, volumeSampler, uvw).r;

        if (density > uniforms.densityThreshold) {
            // Transfer function color mapping
            var color = vec4<f32>(0.2, 0.6, 1.0, density * 0.1); // Default blue
            if (density > 0.5) {
                color = vec4<f32>(1.0, 0.5, 0.0, density * 0.2); // Iron orange
            }
            if (density > 1.5) {
                color = vec4<f32>(1.0, 0.1, 0.1, density * 0.4); // Lead red
            }

            // Alpha blending
            accumulatedColor = accumulatedColor + (1.0 - accumulatedColor.a) * color;
            if (accumulatedColor.a >= 0.95) {
                break;
            }
        }

        samplePos = samplePos + rayDir * uniforms.stepSize;
    }

    return accumulatedColor;
}
