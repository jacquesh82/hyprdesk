// Lentille de verre de hyprdesk (MLG-275) : ce qui est déjà dessiné derrière
// la surface est flouté (niveaux de mip), réfracté au bord (loi de
// Snell-Descartes sur un bord arrondi), décomposé (aberration chromatique),
// teinté du fond de la surface et éclairé sur son liseré.
//
// Inspiré d'iced_glass (Max Bergmark, licence MIT,
// https://github.com/maxbergmark/iced_glass) : fonctions de distance signée
// d'une boîte arrondie et calcul du décalage de réfraction.

struct Uniforms {
    // largeur, hauteur (px physiques), rayon des coins, largeur du bord
    // réfractant.
    shape: vec4<f32>,
    // indice de réfraction, aberration, niveau de flou (lod), liseré.
    optics: vec4<f32>,
    // teinte du fond (non prémultipliée), alpha = opacité de la surface.
    tint: vec4<f32>,
    // pointeur : x, y (px dans la surface), intensité, rayon.
    pointer: vec4<f32>,
    // rayons des coins : haut gauche, haut droit, bas droit, bas gauche.
    radii: vec4<f32>,
};

@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@group(1) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
    );
    let pos = positions[i];
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 0.5 - pos.y * 0.5);
    return out;
}

// Réduction d'un niveau de mip au suivant (filtre linéaire, 4 prises).
@fragment
fn fs_down(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(image));
    let o = texel * 0.5;
    return 0.25 * (
        textureSampleLevel(image, image_sampler, in.uv + vec2<f32>(-o.x, -o.y), 0.0) +
        textureSampleLevel(image, image_sampler, in.uv + vec2<f32>(o.x, -o.y), 0.0) +
        textureSampleLevel(image, image_sampler, in.uv + vec2<f32>(-o.x, o.y), 0.0) +
        textureSampleLevel(image, image_sampler, in.uv + vec2<f32>(o.x, o.y), 0.0)
    );
}

// Distance signée d'une boîte et son gradient (normale sortante).
fn sdg_box(p: vec2<f32>, b: vec2<f32>) -> vec3<f32> {
    let w = abs(p) - b;
    let s = vec2<f32>(select(1.0, -1.0, p.x < 0.0), select(1.0, -1.0, p.y < 0.0));
    let g = max(w.x, w.y);
    let q = max(w, vec2<f32>(0.0));
    let l = length(q);
    let inner = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), w.x > w.y);
    let grad = s * select(inner, q / max(l, 1e-6), g > 0.0);
    return vec3<f32>(select(g, l, g > 0.0), grad);
}

fn sdg_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> vec3<f32> {
    let d = sdg_box(p, b - vec2<f32>(r));
    return vec3<f32>(d.x - r, d.y, d.z);
}

// Décalage de réfraction d'un rayon vertical à travers un bord arrondi de
// rayon `r` et d'épaisseur `h`, d'indice `n`, à la distance `x` du bord.
fn refract_offset(x: f32, r: f32, n: f32, h: f32) -> f32 {
    let x2 = clamp(x + r, -r, r);
    let z = sqrt(max(r * r - x2 * x2, 1e-4));
    let theta = atan(x2 / z);
    let gamma = asin(sin(theta) / n);
    let beta = theta - gamma;
    return -(z + h) * tan(beta);
}

fn sample_at(p: vec2<f32>, size: vec2<f32>, lod: f32) -> vec4<f32> {
    let uv = clamp(p / size + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleLevel(image, image_sampler, uv, lod);
}

@fragment
fn fs_lens(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = u.shape.xy;
    let p0 = (in.uv - vec2<f32>(0.5)) * size;
    // Rayon du coin le plus proche (y vers le bas).
    let corner = select(
        select(u.radii.x, u.radii.y, p0.x > 0.0),
        select(u.radii.w, u.radii.z, p0.x > 0.0),
        p0.y > 0.0
    );
    let r = min(corner, min(size.x, size.y) * 0.5);
    let edge = max(min(u.shape.w, min(size.x, size.y) * 0.5), 0.5);
    let p = (in.uv - vec2<f32>(0.5)) * size;
    let sd = sdg_rounded_box(p, size * 0.5, r);
    let dist = sd.x;
    let grad = sd.yz;
    let aa = max(fwidth(dist), 0.75);
    let coverage = 1.0 - smoothstep(-aa, aa, dist);
    if coverage <= 0.0 {
        return vec4<f32>(0.0);
    }

    let n = max(u.optics.x, 1.0);
    let ab = u.optics.y;
    let lod = u.optics.z;
    let h = edge * 2.0;
    var color: vec4<f32>;
    if dist > -edge && n > 1.0001 {
        let off_g = grad * refract_offset(dist, edge, n, h);
        let g = sample_at(p + off_g, size, lod);
        if ab > 0.0 {
            let rr = sample_at(p + grad * refract_offset(dist, edge, max(n - ab, 1.0), h), size, lod);
            let bb = sample_at(p + grad * refract_offset(dist, edge, n + ab, h), size, lod);
            color = vec4<f32>(rr.r, g.g, bb.b, g.a);
        } else {
            color = g;
        }
    } else {
        color = sample_at(p, size, lod);
    }

    // Fond de la surface posé sur ce qui est derrière (prémultiplié).
    let t = u.tint;
    var glass = color * (1.0 - t.a) + vec4<f32>(t.rgb * t.a, t.a);

    // Liseré : lumière venue d'en haut à gauche, sur le bord.
    let sun = normalize(vec2<f32>(-0.5, -1.0));
    let facing = pow(max(dot(grad, sun), 0.0), 2.0);
    let rim = (1.0 - smoothstep(0.0, 2.5, -dist)) * facing * u.optics.w;
    glass = vec4<f32>(mix(glass.rgb, vec3<f32>(glass.a), rim), glass.a);

    // Reflet qui suit le pointeur.
    if u.pointer.z > 0.0 {
        let d = distance(in.uv * size, u.pointer.xy);
        let k = exp(-(d * d) / max(u.pointer.w * u.pointer.w, 1.0)) * u.pointer.z;
        glass = vec4<f32>(mix(glass.rgb, vec3<f32>(glass.a), k), glass.a);
    }
    return glass * coverage;
}
