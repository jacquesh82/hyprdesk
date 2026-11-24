//! Vraie réfraction (MLG-275) : lentille de verre dessinée par le GPU.
//!
//! La primitive copie ce que la surface a déjà dessiné derrière elle (fond
//! d'écran du bureau, icônes, tranche de fond peinte sous la barre), en
//! tire une chaîne de mip (flou), puis la réfracte au bord, la teinte du
//! fond de la surface et l'éclaire (`lens.wgsl`). Approche d'iced_glass
//! (MIT), réécrite pour hyprdesk.
//!
//! Hyprdesk ne voit jamais les fenêtres : derrière une surface posée sur
//! des fenêtres, la copie est transparente et la lentille se réduit à la
//! teinte. Sans copie possible (surface sans `COPY_SRC`) même chose ; en
//! rendu logiciel (tiny-skia) la lentille n'est pas employée du tout
//! ([`available`]).

use std::collections::{HashMap, HashSet};

use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{Color, Rectangle, mouse};

use crate::theme_engine::design::Glass;

/// Niveaux de mip au plus (le dernier floute ~16 px).
const MIPS: u32 = 5;

/// La lentille peut être dessinée : rendu matériel (wgpu).
pub fn available() -> bool {
    // Inconnu (aperçu, avant la première surface) : wgpu, le choix
    // d'iced quand une carte répond.
    crate::render::running_hardware() != Some(false)
}

/// Lentille d'une surface : forme, verre du thème, fond.
#[derive(Debug, Clone)]
pub struct Lens {
    pub glass: Glass,
    pub radius: iced::border::Radius,
    /// Fond de la surface (alpha = opacité).
    pub tint: Color,
    /// Flou de ce qui est derrière.
    pub blur: bool,
    /// La lentille remplace ce qui est sous elle au lieu de s'y poser :
    /// hors de sa forme arrondie il ne reste rien. Pour une surface dont le
    /// fond est une tranche rectangulaire peinte dessous (barre, dock,
    /// menus, Réglages), qui dépasserait sinon aux coins.
    pub erase: bool,
}

impl<M> shader::Program<M> for Lens {
    type State = ();
    type Primitive = Primitive;

    fn draw(&self, _state: &(), cursor: mouse::Cursor, bounds: Rectangle) -> Primitive {
        let pointer = (self.glass.pointer_response && self.glass.highlight > 0.0)
            .then(|| cursor.position_in(bounds))
            .flatten();
        Primitive {
            lens: self.clone(),
            pointer: pointer.map(|p| (p.x, p.y)),
            id: std::sync::atomic::AtomicU64::new(u64::MAX),
        }
    }
}

/// Primitive GPU d'une lentille.
#[derive(Debug)]
pub struct Primitive {
    lens: Lens,
    pointer: Option<(f32, f32)>,
    /// Instance de textures, posée par `prepare`.
    id: std::sync::atomic::AtomicU64,
}

impl Clone for Primitive {
    fn clone(&self) -> Self {
        Self {
            lens: self.lens.clone(),
            pointer: self.pointer,
            id: std::sync::atomic::AtomicU64::new(
                self.id.load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

/// Uniformes de `lens.wgsl` (quatre `vec4<f32>`).
#[derive(Clone, Copy)]
struct Uniforms {
    shape: [f32; 4],
    optics: [f32; 4],
    tint: [f32; 4],
    pointer: [f32; 4],
    radii: [f32; 4],
}

impl Uniforms {
    const SIZE: u64 = 80;

    fn bytes(&self) -> Vec<u8> {
        [self.shape, self.optics, self.tint, self.pointer, self.radii]
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect()
    }
}

/// Textures d'une lentille : copie de l'arrière-plan et sa chaîne de mip.
struct Instance {
    size: (u32, u32),
    bounds: Rectangle<u32>,
    texture: wgpu::Texture,
    /// Vue de chaque niveau (source de la réduction suivante).
    levels: Vec<wgpu::TextureView>,
    /// Groupe « un seul niveau » par niveau (réduction), puis toute la chaîne.
    level_groups: Vec<wgpu::BindGroup>,
    full_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    mips: u32,
}

/// Pipelines partagés par toutes les lentilles.
pub struct Pipeline {
    format: wgpu::TextureFormat,
    sampler: wgpu::Sampler,
    textures_layout: wgpu::BindGroupLayout,
    uniforms_layout: wgpu::BindGroupLayout,
    down: wgpu::RenderPipeline,
    lens: wgpu::RenderPipeline,
    /// Même dessin, sans mélange : la lentille remplace la cible.
    lens_erase: wgpu::RenderPipeline,
    instances: HashMap<u64, Instance>,
    live: HashSet<u64>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("lens::Pipeline").finish_non_exhaustive()
    }
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hyprdesk.lens"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lens.wgsl").into()),
        });
        let textures_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hyprdesk.lens.textures"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let uniforms_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hyprdesk.lens.uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hyprdesk.lens.sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let make = |label: &str, layouts: &[&wgpu::BindGroupLayout], entry: &str, blend| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let down = make("hyprdesk.lens.down", &[&textures_layout], "fs_down", None);
        let lens = make(
            "hyprdesk.lens.draw",
            &[&textures_layout, &uniforms_layout],
            "fs_lens",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let lens_erase = make(
            "hyprdesk.lens.erase",
            &[&textures_layout, &uniforms_layout],
            "fs_lens",
            Some(wgpu::BlendState::REPLACE),
        );
        Self {
            format,
            sampler,
            textures_layout,
            uniforms_layout,
            down,
            lens,
            lens_erase,
            instances: HashMap::new(),
            live: HashSet::new(),
        }
    }

    fn trim(&mut self) {
        let live = std::mem::take(&mut self.live);
        self.instances.retain(|id, _| live.contains(id));
    }
}

impl Pipeline {
    fn instance(&mut self, device: &wgpu::Device, id: u64, size: (u32, u32)) -> &mut Instance {
        let stale = self.instances.get(&id).is_none_or(|i| i.size != size);
        if stale {
            let mips = MIPS
                .min(32 - size.0.min(size.1).max(1).leading_zeros())
                .max(1);
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("hyprdesk.lens.copy"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let levels: Vec<_> = (0..mips)
                .map(|l| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: l,
                        mip_level_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect();
            let group = |view: &wgpu::TextureView| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("hyprdesk.lens.textures"),
                    layout: &self.textures_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                })
            };
            let level_groups = levels.iter().map(group).collect();
            let full = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let full_group = group(&full);
            let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("hyprdesk.lens.uniforms"),
                size: Uniforms::SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("hyprdesk.lens.uniforms"),
                layout: &self.uniforms_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                }],
            });
            self.instances.insert(
                id,
                Instance {
                    size,
                    bounds: Rectangle::default(),
                    texture,
                    levels,
                    level_groups,
                    full_group,
                    uniforms,
                    uniform_group,
                    mips,
                },
            );
        }
        self.instances.get_mut(&id).expect("instance créée")
    }
}

/// Composante sRGB vers linéaire (cible au format sRGB).
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl Primitive {
    /// Identité stable d'une lentille d'une image à l'autre : sa place.
    fn id(bounds: &Rectangle<u32>) -> u64 {
        (u64::from(bounds.x) << 48)
            ^ (u64::from(bounds.y) << 32)
            ^ (u64::from(bounds.width) << 16)
            ^ u64::from(bounds.height)
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let scale = viewport.scale_factor();
        let phys = Rectangle {
            x: (bounds.x * scale).round().max(0.0) as u32,
            y: (bounds.y * scale).round().max(0.0) as u32,
            width: (bounds.width * scale).round().max(1.0) as u32,
            height: (bounds.height * scale).round().max(1.0) as u32,
        };
        let id = Self::id(&phys);
        self.id.store(id, std::sync::atomic::Ordering::Relaxed);
        pipeline.live.insert(id);
        let srgb = pipeline.format.is_srgb();
        let l = &self.lens;
        let g = &l.glass;
        let inst = pipeline.instance(device, id, (phys.width, phys.height));
        inst.bounds = phys;
        let tint = if srgb {
            [
                linear(l.tint.r),
                linear(l.tint.g),
                linear(l.tint.b),
                l.tint.a,
            ]
        } else {
            [l.tint.r, l.tint.g, l.tint.b, l.tint.a]
        };
        let lod = if l.blur {
            (inst.mips.saturating_sub(1) as f32).min(3.2)
        } else {
            0.0
        };
        let (px, py) = self.pointer.unwrap_or((0.0, 0.0));
        let u = Uniforms {
            shape: [
                phys.width as f32,
                phys.height as f32,
                0.0,
                g.refraction_width.max(1.0) * scale,
            ],
            optics: [
                1.0 + 0.6 * g.refraction,
                0.04 * g.refraction,
                lod,
                (g.highlight * 2.5).min(1.0),
            ],
            tint,
            pointer: [
                px * scale,
                py * scale,
                if self.pointer.is_some() {
                    g.highlight * 0.5
                } else {
                    0.0
                },
                (phys.width.min(phys.height) as f32 * 0.35).max(40.0),
            ],
            radii: [
                l.radius.top_left * scale,
                l.radius.top_right * scale,
                l.radius.bottom_right * scale,
                l.radius.bottom_left * scale,
            ],
        };
        queue.write_buffer(&inst.uniforms, 0, &u.bytes());
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip: &Rectangle<u32>,
    ) {
        let id = self.id.load(std::sync::atomic::Ordering::Relaxed);
        let Some(inst) = pipeline.instances.get(&id) else {
            return;
        };
        let source = target.texture();
        let copyable = source.usage().contains(wgpu::TextureUsages::COPY_SRC)
            && source.format() == pipeline.format;
        // Fond : copie de ce qui est déjà dessiné, sinon transparent (la
        // lentille se réduit alors à la teinte de la surface).
        if copyable {
            let w = clip.width.min(source.width().saturating_sub(clip.x));
            let h = clip.height.min(source.height().saturating_sub(clip.y));
            if w > 0 && h > 0 {
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: source,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: clip.x,
                            y: clip.y,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &inst.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: clip.x.saturating_sub(inst.bounds.x).min(inst.size.0 - 1),
                            y: clip.y.saturating_sub(inst.bounds.y).min(inst.size.1 - 1),
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: w.min(
                            inst.size.0 - clip.x.saturating_sub(inst.bounds.x).min(inst.size.0 - 1),
                        ),
                        height: h.min(
                            inst.size.1 - clip.y.saturating_sub(inst.bounds.y).min(inst.size.1 - 1),
                        ),
                        depth_or_array_layers: 1,
                    },
                );
            }
        } else {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hyprdesk.lens.clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &inst.levels[0],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        // Chaîne de mip : chaque niveau réduit le précédent.
        for level in 1..inst.mips as usize {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hyprdesk.lens.down"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &inst.levels[level],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline.down);
            pass.set_bind_group(0, &inst.level_groups[level - 1], &[]);
            pass.draw(0..6, 0..1);
        }
        // Lentille posée sur la cible.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hyprdesk.lens.draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_viewport(
            inst.bounds.x as f32,
            inst.bounds.y as f32,
            inst.bounds.width as f32,
            inst.bounds.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(clip.x, clip.y, clip.width, clip.height);
        pass.set_pipeline(if self.lens.erase {
            &pipeline.lens_erase
        } else {
            &pipeline.lens
        });
        pass.set_bind_group(0, &inst.full_group, &[]);
        pass.set_bind_group(1, &inst.uniform_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
