struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_position: vec2<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.frag_position = in.position.xy;
    return out;
}

struct CameraUniform {
    offset: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32
};

@group(0) @binding(0)
    var<uniform> camera: CameraUniform;

let MAX_ITER : u32 = 100u;


fn square_length(z: vec2<f32>) -> f32{
    return dot(z, z);

}

fn distance_to_line(p: vec2<f32>, line_point: vec2<f32>, direction:vec2<f32>) -> f32{
    let direction = normalize(direction);
    let v = line_point - p;
    return square_length(dot(v, direction)*direction - v);
}

fn sigmoid(x: f32, center: f32, zoom: f32) -> f32 {
    return 1.0 / (1.0 + (exp(-zoom*(x-center))));
}

struct Julia {
    d: f32,
    n: u32
};

fn julia_standard(c :vec2<f32>, z: vec2<f32>) -> Julia{
    var z = z;
    var dz = vec2<f32>(0.0);
    var n = 0u;
    loop{
        if square_length(z) > 400.0 || n >= MAX_ITER {
            break
        }

        dz = 2.0*vec2(z.x*dz.x-z.y*dz.y, z.x*dz.y + z.y*dz.x ) + vec2(1.0,0.0);
        z = vec2(z[0]*z[0] - z[1]*z[1], 2.*z[1]*z[0]) + c;
        n++;
    }

    var a: Julia;
    a.d = sqrt( dot(z,z)/dot(dz,dz) )*log(dot(z,z));
    a.n = n;
    return a;
}

fn square(p: vec2<f32>) -> f32 {
    let j = julia_standard(vec2<f32>(0.35, 0.5), p/1.1 + vec2<f32>(0.3, -0.5));
    let c = clamp( pow(9.0*j.d/6.0,0.12), 0.0, 1.0 );
    return c;
}


// maldelbrot and julia sets
// mandelbrot set is given by julia(position.x, position.y, 0., 0.);
// julia set is given by julia(cx, cy, position.x, position.y); for cx, cy given
fn julia_trap(c :vec2<f32>, z: vec2<f32>) -> vec4<f32>{
    var z = z;
    var old = z;
    let p = vec2<f32>(1.0, 0.1);

    var dist = 100.;
    var s = vec3<f32>(-0.17, 0.6, 2.);
    var n = 0u;
    loop{
        if square_length(z) > 4.0 || n >= MAX_ITER {
            break
        }

        old = z;
        z = vec2(z[0]*z[0] - z[1]*z[1], 2.*z[1]*z[0]) + c;
        let d = square(s.z*z+s.xz);
        dist = min(dist, d);
        n++;
    }

    let f = (f32(n) + (4.0 - square_length(old)) / (0.1 + abs(square_length(z)  - square_length(old)))) / f32(MAX_ITER);
    let f = clamp(f, 0.01, 1.0);
    let g = clamp(pow(7.0*f/6.0,0.5), 0.0, 1.0 );
    let bg = 0.6*vec3<f32>(0.6, 0.4, 0.9) * (0.2 + sqrt(f));
    let bg2 = 0.2*vec3<f32>(0.6, 0.7, 0.6) * (0.2 + sqrt(f));
    let fg = 0.5*vec3<f32>(0.2, 0.8, 0.9) * (0.3 + sqrt(1.-f));
    let color = mix(fg, bg, dist );
    let color = mix(color*bg2, color, g);
    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var frag_position = in.frag_position;
    frag_position.x *= camera.aspect_ratio;

    let position = camera.zoom * frag_position - camera.offset;

    return julia_trap(position, vec2<f32>(0.1)); 
}
