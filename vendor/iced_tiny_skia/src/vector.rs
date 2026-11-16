use crate::core::svg::{Data, Handle};
use crate::core::{Color, Rectangle, Size};

use resvg::usvg;
use rustc_hash::{FxHashMap, FxHashSet};
use tiny_skia::Transform;

use std::cell::RefCell;
use std::collections::hash_map;
use std::fs;
use std::panic;
use std::sync::Arc;

#[derive(Debug)]
pub struct Pipeline {
    cache: RefCell<Cache>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(Cache::default()),
        }
    }

    pub fn viewport_dimensions(&self, handle: &Handle) -> Size<u32> {
        self.cache
            .borrow_mut()
            .viewport_dimensions(handle)
            .unwrap_or(Size::new(0, 0))
    }

    pub fn draw(
        &mut self,
        handle: &Handle,
        color: Option<Color>,
        bounds: Rectangle,
        opacity: f32,
        pixels: &mut tiny_skia::PixmapMut<'_>,
        transform: Transform,
        clip_mask: Option<&tiny_skia::Mask>,
    ) {
        if let Some(image) = self.cache.borrow_mut().draw(
            handle,
            color,
            Size::new(
                (bounds.width * transform.sx) as u32,
                (bounds.height * transform.sy) as u32,
            ),
        ) {
            pixels.draw_pixmap(
                (bounds.x * transform.sx) as i32,
                (bounds.y * transform.sy) as i32,
                image,
                &tiny_skia::PixmapPaint {
                    opacity,
                    ..tiny_skia::PixmapPaint::default()
                },
                Transform::default(),
                clip_mask,
            );
        }
    }

    pub fn trim_cache(&mut self) {
        self.cache.borrow_mut().trim();
    }
}

#[derive(Default)]
struct Cache {
    trees: FxHashMap<u64, Option<resvg::usvg::Tree>>,
    tree_hits: FxHashSet<u64>,
    rasters: FxHashMap<RasterKey, tiny_skia::Pixmap>,
    raster_hits: FxHashSet<RasterKey>,
}

/// hyprdesk : base de polices des SVG, commune à tout le processus.
///
/// L'original en chargeait une par cache, c'est-à-dire par moteur de rendu,
/// donc par surface : chaque tiroir ouvert relisait toutes les polices du
/// système sur le fil principal (MLG-156). Elle n'est chargée qu'une fois,
/// et seulement pour un SVG qui porte du texte (ou compressé, illisible
/// avant décodage) ; les icônes reçoivent une base vide.
fn shared_fontdb(data: &[u8]) -> Arc<usvg::fontdb::Database> {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    static SYSTEM: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let gzip = data.starts_with(&[0x1f, 0x8b]);
    if gzip || data.windows(5).any(|w| w == b"<text") {
        SYSTEM
            .get_or_init(|| {
                let mut fontdb = usvg::fontdb::Database::new();
                fontdb.load_system_fonts();
                Arc::new(fontdb)
            })
            .clone()
    } else {
        EMPTY
            .get_or_init(|| Arc::new(usvg::fontdb::Database::new()))
            .clone()
    }
}

fn svg_options(data: &[u8]) -> usvg::Options<'static> {
    usvg::Options {
        fontdb: shared_fontdb(data),
        ..usvg::Options::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RasterKey {
    id: u64,
    color: Option<[u8; 4]>,
    size: Size<u32>,
}

impl Cache {
    fn load(&mut self, handle: &Handle) -> Option<&usvg::Tree> {
        let id = handle.id();

        // hyprdesk : base de polices commune, voir `shared_fontdb` (MLG-156).
        if let hash_map::Entry::Vacant(entry) = self.trees.entry(id) {
            let svg = match handle.data() {
                Data::Path(path) => {
                    fs::read_to_string(path).ok().and_then(|contents| {
                        let options = svg_options(contents.as_bytes());
                        usvg::Tree::from_str(&contents, &options).ok()
                    })
                }
                Data::Bytes(bytes) => {
                    let options = svg_options(bytes);
                    usvg::Tree::from_data(bytes, &options).ok()
                }
            };

            let _ = entry.insert(svg);
        }

        let _ = self.tree_hits.insert(id);
        self.trees.get(&id).unwrap().as_ref()
    }

    fn viewport_dimensions(&mut self, handle: &Handle) -> Option<Size<u32>> {
        let tree = self.load(handle)?;
        let size = tree.size();

        Some(Size::new(size.width() as u32, size.height() as u32))
    }

    fn draw(
        &mut self,
        handle: &Handle,
        color: Option<Color>,
        size: Size<u32>,
    ) -> Option<tiny_skia::PixmapRef<'_>> {
        if size.width == 0 || size.height == 0 {
            return None;
        }

        let key = RasterKey {
            id: handle.id(),
            color: color.map(Color::into_rgba8),
            size,
        };

        #[allow(clippy::map_entry)]
        if !self.rasters.contains_key(&key) {
            let tree = self.load(handle)?;

            let mut image = tiny_skia::Pixmap::new(size.width, size.height)?;

            let tree_size = tree.size().to_int_size();

            let target_size = if size.width > size.height {
                tree_size.scale_to_width(size.width)
            } else {
                tree_size.scale_to_height(size.height)
            };

            let transform = if let Some(target_size) = target_size {
                let tree_size = tree_size.to_size();
                let target_size = target_size.to_size();

                tiny_skia::Transform::from_scale(
                    target_size.width() / tree_size.width(),
                    target_size.height() / tree_size.height(),
                )
            } else {
                tiny_skia::Transform::default()
            };

            // SVG rendering can panic on malformed or complex vectors.
            // We catch panics to prevent crashes and continue gracefully.
            let render = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                resvg::render(tree, transform, &mut image.as_mut());
            }));

            if let Err(error) = render {
                log::warn!("SVG rendering for {handle:?} panicked: {error:?}");
            }

            if let Some([r, g, b, _]) = key.color {
                // Apply color filter
                for pixel in
                    bytemuck::cast_slice_mut::<u8, u32>(image.data_mut())
                {
                    *pixel = bytemuck::cast(
                        tiny_skia::ColorU8::from_rgba(
                            b,
                            g,
                            r,
                            (*pixel >> 24) as u8,
                        )
                        .premultiply(),
                    );
                }
            } else {
                // Swap R and B channels for `softbuffer` presentation
                for pixel in
                    bytemuck::cast_slice_mut::<u8, u32>(image.data_mut())
                {
                    *pixel = *pixel & 0xFF00_FF00
                        | ((0x0000_00FF & *pixel) << 16)
                        | ((0x00FF_0000 & *pixel) >> 16);
                }
            }

            let _ = self.rasters.insert(key, image);
        }

        let _ = self.raster_hits.insert(key);
        self.rasters.get(&key).map(tiny_skia::Pixmap::as_ref)
    }

    fn trim(&mut self) {
        self.trees.retain(|key, _| self.tree_hits.contains(key));
        self.rasters.retain(|key, _| self.raster_hits.contains(key));

        self.tree_hits.clear();
        self.raster_hits.clear();
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("tree_hits", &self.tree_hits)
            .field("rasters", &self.rasters)
            .field("raster_hits", &self.raster_hits)
            .finish_non_exhaustive()
    }
}
