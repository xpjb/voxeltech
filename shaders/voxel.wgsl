struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec4f,
}

struct VertexOutput {
    @location(0) @interpolate(flat) color: vec4f,
    @location(1) normal: vec3f,
    @builtin(position) clip_position: vec4f,
}

struct CameraUniforms {
    view_proj: mat4x4f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<storage, read> instances: array<mat4x4f>;

struct InstanceInput {
    @builtin(instance_index) instance_index: u32,
}

@vertex
fn vs_main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    let model = instances[inst.instance_index];
    let world_pos = model * vec4f(in.position, 1.0);
    let world_normal = (model * vec4f(in.normal, 0.0)).xyz;
    var out: VertexOutput;
    out.color = in.color;
    out.normal = world_normal;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let light_dir = normalize(vec3f(0.5, 1.0, 0.3));
    let NdotL = max(0.0, dot(normalize(in.normal), light_dir));
    let ambient = 0.3;
    let diffuse = NdotL * 0.7;
    // Output linear RGB - Bgra8UnormSrgb target; GPU handles format layout
    let rgb = in.color.rgb * (ambient + diffuse);
    return vec4f(rgb.r, rgb.g, rgb.b, in.color.a);
}
