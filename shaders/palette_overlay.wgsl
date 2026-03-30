struct VertexIn {
    @location(0) pos: vec2f,
    @location(1) color: vec4f,
}

struct VertexOut {
    @builtin(position) clip: vec4f,
    @location(0) color: vec4f,
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var o: VertexOut;
    o.clip = vec4f(v.pos, 0.0, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_main(i: VertexOut) -> @location(0) vec4f {
    return i.color;
}
