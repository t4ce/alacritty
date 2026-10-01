//! Alacritty's font value types and native TRUEOS provider.
//! Value types retain the crossfont-facing frontend shape (Apache-2.0), but
//! glyph production is kernel-owned and returns opaque UI4 sprite resources.
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Max font size in pt.
///
/// The value is picked based on `u32` max, since we use 6 digits for fract.
const MAX_FONT_PT_SIZE: f32 = 3999.;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontDesc {
    name: String,
    style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slant {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    Normal,
    Bold,
}

/// Style of font.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Style {
    Specific(String),
    Description { slant: Slant, weight: Weight },
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Style::Specific(ref s) => f.write_str(s),
            Style::Description { slant, weight } => {
                write!(f, "slant={slant:?}, weight={weight:?}")
            },
        }
    }
}

impl FontDesc {
    pub fn new<S>(name: S, style: Style) -> FontDesc
    where
        S: Into<String>,
    {
        FontDesc { name: name.into(), style }
    }
}

impl fmt::Display for FontDesc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.name, self.style)
    }
}

/// Identifier for a Font for use in maps/etc.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct FontKey {
    token: u32,
}

impl FontKey {
    /// Get next font key for given size.
    ///
    /// The generated key will be globally unique.
    pub fn next() -> FontKey {
        static TOKEN: AtomicUsize = AtomicUsize::new(0);

        FontKey { token: TOKEN.fetch_add(1, Ordering::SeqCst) as _ }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct GlyphKey {
    pub character: char,
    pub font_key: FontKey,
    pub size: Size,
}

/// Font size stored as base and fraction.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(u32);

impl Size {
    /// Create a new `Size` from a f32 size in points.
    ///
    /// The font size is automatically clamped to supported range of `[1.; 3999.]` pt.
    pub fn new(size: f32) -> Size {
        let size = size.clamp(1., MAX_FONT_PT_SIZE);
        Size((size * Self::factor()) as u32)
    }

    /// Create a new `Size` from px.
    ///
    /// The value will be clamped to the pt range of [`Size::new`].
    pub fn from_px(size: f32) -> Self {
        let pt = size * 72. / 96.;
        Size::new(pt)
    }

    /// Scale font size by the given amount.
    pub fn scale(self, scale: f32) -> Self {
        Self::new(self.as_pt() * scale)
    }

    /// Get size in `px`.
    pub fn as_px(self) -> f32 {
        self.as_pt() * 96. / 72.
    }

    /// Get the size in `pt`.
    pub fn as_pt(self) -> f32 {
        (f64::from(self.0) / Size::factor() as f64) as f32
    }

    /// Scale factor between font "Size" type and point size.
    #[inline]
    fn factor() -> f32 {
        1_000_000.
    }
}

#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    pub character: char,
    pub width: i32,
    pub height: i32,
    pub top: i32,
    pub left: i32,
    pub advance: (i32, i32),
    pub buffer: BitmapBuffer,
}

#[derive(Clone, Debug)]
pub enum BitmapBuffer {
    /// Window-owned kernel glyph; never read back to a CPU atlas.
    Native(NativeGlyph),
    /// RGB alphamask.
    Rgb(Vec<u8>),

    /// RGBA pixels with premultiplied alpha.
    Rgba(Vec<u8>),
}

impl Default for RasterizedGlyph {
    fn default() -> RasterizedGlyph {
        RasterizedGlyph {
            character: ' ',
            width: 0,
            height: 0,
            top: 0,
            left: 0,
            advance: (0, 0),
            buffer: BitmapBuffer::Rgb(Vec::new()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Metrics {
    pub average_advance: f64,
    pub line_height: f64,
    pub descent: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikeout_position: f32,
    pub strikeout_thickness: f32,
}

/// Errors occuring when using the rasterizer.
#[derive(Debug)]
pub enum Error {
    /// Native producer is pending or temporarily full; do not cache a blank glyph.
    Pending,
    /// Unable to find a font matching the description.
    FontNotFound(FontDesc),

    /// Unable to find metrics for a font face.
    MetricsNotFound,

    /// The glyph could not be found in any font.
    MissingGlyph(RasterizedGlyph),

    /// Requested an operation with a FontKey that isn't known to the rasterizer.
    UnknownFontKey,

    /// Error from platfrom's font system.
    PlatformError(String),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Pending => f.write_str("native font producer is pending"),
            Error::FontNotFound(font) => write!(f, "font {font:?} not found"),
            Error::MissingGlyph(glyph) => {
                write!(f, "glyph for character {:?} not found", glyph.character)
            },
            Error::UnknownFontKey => f.write_str("invalid font key"),
            Error::MetricsNotFound => f.write_str("metrics not found"),
            Error::PlatformError(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeGlyph {
    pub window: u32,
    pub sprite: u32,
    pub ticket: u64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct NativeMetrics {
    version: u32,
    font_id: u32,
    flags: u32,
    pixels: f32,
    cell_advance: f32,
    line_height: f32,
    ascent: f32,
    descent: f32,
    underline_position: f32,
    underline_thickness: f32,
    strikeout_position: f32,
    strikeout_thickness: f32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct NativeStatus {
    state: u32,
    sprite_id: u32,
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
}
const _: () = assert!(std::mem::size_of::<NativeMetrics>() == 48);
const _: () = assert!(std::mem::size_of::<NativeStatus>() == 24);

trait Host {
    fn metrics(&self, window: u32, face: u32, pixels: f32) -> Result<NativeMetrics, i32>;
    fn request(&self, window: u32, face: u32, scalar: char, pixels: f32) -> Result<u64, i32>;
    fn status(&self, window: u32, ticket: u64) -> Result<NativeStatus, i32>;
}
struct Kernel;
unsafe extern "C" {
    fn trueos_cabi_ui4_scene_font_metrics_v1(
        window: u32,
        font: u32,
        pixels: f32,
        out: *mut NativeMetrics,
    ) -> i32;
    fn trueos_cabi_ui4_scene_font_sprite_request_v1(
        window: u32,
        font: u32,
        scalar: u32,
        pixels: f32,
        color: u32,
        ticket: *mut u64,
    ) -> i32;
    fn trueos_cabi_ui4_scene_font_sprite_status_v1(
        window: u32,
        ticket: u64,
        out: *mut NativeStatus,
    ) -> i32;
}
impl Host for Kernel {
    fn metrics(&self, window: u32, face: u32, pixels: f32) -> Result<NativeMetrics, i32> {
        let mut result = NativeMetrics::default();
        let rc =
            unsafe { trueos_cabi_ui4_scene_font_metrics_v1(window, face, pixels, &mut result) };
        if rc == 0 { Ok(result) } else { Err(rc) }
    }
    fn request(&self, window: u32, face: u32, scalar: char, pixels: f32) -> Result<u64, i32> {
        let mut ticket = 0;
        let rc = unsafe {
            trueos_cabi_ui4_scene_font_sprite_request_v1(
                window,
                face,
                scalar as u32,
                pixels,
                u32::MAX,
                &mut ticket,
            )
        };
        if rc == 0 && ticket != 0 { Ok(ticket) } else { Err(if rc == 0 { -5 } else { rc }) }
    }
    fn status(&self, window: u32, ticket: u64) -> Result<NativeStatus, i32> {
        let mut result = NativeStatus::default();
        let rc =
            unsafe { trueos_cabi_ui4_scene_font_sprite_status_v1(window, ticket, &mut result) };
        if rc == 0 { Ok(result) } else { Err(rc) }
    }
}

pub struct Rasterizer {
    inner: Provider<Kernel>,
}
impl Rasterizer {
    pub fn for_window(window: u32) -> Result<Self, Error> {
        if window == 0 {
            return Err(Error::PlatformError("native font requires a live UI4 window".into()));
        }
        Ok(Self { inner: Provider::new(window, Kernel) })
    }
    pub fn load_font(&mut self, desc: &FontDesc, size: Size) -> Result<FontKey, Error> {
        self.inner.load_font(desc, size)
    }
    pub fn metrics(&self, key: FontKey, size: Size) -> Result<Metrics, Error> {
        self.inner.metrics(key, size)
    }
    pub fn get_glyph(&mut self, key: GlyphKey) -> Result<RasterizedGlyph, Error> {
        self.inner.get_glyph(key)
    }
    pub fn take_redraw_request(&mut self) -> bool {
        self.inner.take_redraw_request()
    }
}
struct Provider<H> {
    host: H,
    window: u32,
    faces: HashMap<FontKey, u32>,
    pending: HashMap<GlyphKey, u64>,
    busy: HashMap<GlyphKey, u16>,
    retry: bool,
}
impl<H: Host> Provider<H> {
    fn new(window: u32, host: H) -> Self {
        Self {
            host,
            window,
            faces: HashMap::new(),
            pending: HashMap::new(),
            busy: HashMap::new(),
            retry: false,
        }
    }
    fn load_font(&mut self, desc: &FontDesc, size: Size) -> Result<FontKey, Error> {
        let regular = match &desc.style {
            Style::Description { slant: Slant::Normal, weight: Weight::Normal } => true,
            Style::Specific(style) => {
                matches!(style.to_ascii_lowercase().as_str(), "regular" | "normal" | "roman")
            },
            _ => false,
        };
        let requested = match desc.name.to_ascii_lowercase().as_str() {
            "monospace" | "juliamono" | "julia-mono" => 4,
            "inconsolata" => 3,
            _ => return Err(Error::FontNotFound(desc.clone())),
        };
        if !regular {
            return Err(Error::FontNotFound(desc.clone()));
        }
        let metrics = self.native_metrics(requested, size)?;
        // Pin the resolved identity. Optional JuliaMono must not change face
        // halfway through the lifetime of a font key if it appears later.
        if !matches!(metrics.font_id, 3 | 4) {
            return Err(Error::MetricsNotFound);
        }
        if let Some((key, _)) = self.faces.iter().find(|(_, face)| **face == metrics.font_id) {
            return Ok(*key);
        }
        let key = FontKey::next();
        self.faces.insert(key, metrics.font_id);
        Ok(key)
    }
    fn native_metrics(&self, face: u32, size: Size) -> Result<NativeMetrics, Error> {
        let px = size.as_px();
        if !(4.0..=256.0).contains(&px) {
            return Err(Error::PlatformError("TRUEOS font size must be 4–256 pixels".into()));
        }
        let m = self.host.metrics(self.window, face, px).map_err(platform_error)?;
        if m.version != 1
            || m.pixels != px
            || m.flags & !1 != 0
            || ![
                m.cell_advance,
                m.line_height,
                m.ascent,
                m.descent,
                m.underline_position,
                m.underline_thickness,
                m.strikeout_position,
                m.strikeout_thickness,
            ]
            .into_iter()
            .all(f32::is_finite)
            || m.cell_advance <= 0.0
            || m.line_height <= 0.0
            || m.ascent <= 0.0
            || m.descent > 0.0
            || m.underline_thickness <= 0.0
            || m.strikeout_thickness <= 0.0
        {
            return Err(Error::MetricsNotFound);
        }
        Ok(m)
    }
    fn metrics(&self, key: FontKey, size: Size) -> Result<Metrics, Error> {
        let face = *self.faces.get(&key).ok_or(Error::UnknownFontKey)?;
        let m = self.native_metrics(face, size)?;
        if m.font_id != face {
            return Err(Error::MetricsNotFound);
        }
        Ok(Metrics {
            average_advance: m.cell_advance as f64,
            line_height: m.line_height as f64,
            descent: m.descent,
            underline_position: m.underline_position,
            underline_thickness: m.underline_thickness,
            strikeout_position: m.strikeout_position,
            strikeout_thickness: m.strikeout_thickness,
        })
    }
    fn get_glyph(&mut self, key: GlyphKey) -> Result<RasterizedGlyph, Error> {
        let face = *self.faces.get(&key.font_key).ok_or(Error::UnknownFontKey)?;
        if !(4.0..=256.0).contains(&key.size.as_px()) {
            return Err(Error::MetricsNotFound);
        }
        if key.character.is_whitespace() {
            return Ok(RasterizedGlyph { character: key.character, ..Default::default() });
        }
        if key.character.is_control() {
            return Err(Error::MissingGlyph(RasterizedGlyph::default()));
        }
        let ticket = match self.pending.get(&key) {
            Some(ticket) => *ticket,
            None => match self.host.request(self.window, face, key.character, key.size.as_px()) {
                Ok(ticket) if ticket != 0 => {
                    self.busy.remove(&key);
                    self.pending.insert(key, ticket);
                    ticket
                },
                Err(-16) => {
                    let attempts = self.busy.entry(key).or_default();
                    *attempts += 1;
                    if *attempts >= 120 {
                        self.busy.remove(&key);
                        return Err(platform_error(-16));
                    }
                    self.retry = true;
                    return Err(Error::Pending);
                },
                Err(error) => return Err(platform_error(error)),
                _ => return Err(platform_error(-5)),
            },
        };
        let status = match self.host.status(self.window, ticket) {
            Ok(status) => status,
            Err(-16) => {
                self.retry = true;
                return Err(Error::Pending);
            },
            Err(error) => {
                self.pending.remove(&key);
                return Err(platform_error(error));
            },
        };
        match status.state {
            1 => {
                self.retry = true;
                Err(Error::Pending)
            },
            2 => {
                self.pending.remove(&key);
                if status.sprite_id == 0
                    || status.width == 0
                    || status.height == 0
                    || status.width > 4096
                    || status.height > 4096
                {
                    return Err(platform_error(-5));
                }
                Ok(RasterizedGlyph {
                    character: key.character,
                    width: status.width as i32,
                    height: status.height as i32,
                    left: status.origin_x,
                    top: (key.size.as_px() - status.origin_y as f32).round() as i32,
                    advance: (0, 0),
                    buffer: BitmapBuffer::Native(NativeGlyph {
                        window: self.window,
                        sprite: status.sprite_id,
                        ticket,
                    }),
                })
            },
            3 => {
                self.pending.remove(&key);
                Err(Error::MissingGlyph(RasterizedGlyph {
                    character: key.character,
                    ..Default::default()
                }))
            },
            _ => {
                self.pending.remove(&key);
                Err(platform_error(-5))
            },
        }
    }
    fn take_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.retry)
    }
}
fn platform_error(code: i32) -> Error {
    Error::PlatformError(format!("TRUEOS font service error {code}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    struct Mock {
        requests: Cell<usize>,
        status: Cell<Result<NativeStatus, i32>>,
        busy: Cell<bool>,
        calls: RefCell<Vec<(u32, u32)>>,
    }
    impl Default for Mock {
        fn default() -> Self {
            Self {
                requests: Cell::new(0),
                status: Cell::new(Ok(NativeStatus { state: 1, ..Default::default() })),
                busy: Cell::new(false),
                calls: RefCell::new(Vec::new()),
            }
        }
    }
    impl Host for Mock {
        fn metrics(&self, window: u32, face: u32, pixels: f32) -> Result<NativeMetrics, i32> {
            self.calls.borrow_mut().push((window, face));
            Ok(NativeMetrics {
                version: 1,
                font_id: 3,
                flags: 1,
                pixels,
                cell_advance: pixels / 2.0,
                line_height: pixels,
                ascent: pixels * 0.8,
                descent: -pixels * 0.2,
                underline_position: -1.0,
                underline_thickness: 1.0,
                strikeout_position: 4.0,
                strikeout_thickness: 1.0,
            })
        }
        fn request(&self, window: u32, face: u32, _: char, _: f32) -> Result<u64, i32> {
            assert_eq!((window, face), (42, 3));
            self.requests.set(self.requests.get() + 1);
            if self.busy.get() { Err(-16) } else { Ok(17) }
        }
        fn status(&self, window: u32, ticket: u64) -> Result<NativeStatus, i32> {
            assert_eq!((window, ticket), (42, 17));
            self.status.get()
        }
    }
    fn setup() -> (Provider<Mock>, GlyphKey) {
        let mut p = Provider::new(42, Mock::default());
        let size = Size::new(11.25);
        let font_key = p
            .load_font(
                &FontDesc::new(
                    "monospace",
                    Style::Description { slant: Slant::Normal, weight: Weight::Normal },
                ),
                size,
            )
            .unwrap();
        (p, GlyphKey { font_key, size, character: 'm' })
    }
    #[test]
    fn default_resolves_and_pins_kernel_fallback() {
        let (p, k) = setup();
        assert_eq!(k.size.as_px(), 15.0);
        assert_eq!(p.metrics(k.font_key, k.size).unwrap().average_advance, 7.5);
        assert_eq!(*p.host.calls.borrow(), vec![(42, 4), (42, 3)]);
    }
    #[test]
    fn pending_deduplicates_then_returns_native_bearings() {
        let (mut p, k) = setup();
        for _ in 0..3 {
            assert!(matches!(p.get_glyph(k), Err(Error::Pending)));
        }
        assert_eq!(p.host.requests.get(), 1);
        assert!(p.take_redraw_request());
        p.host.status.set(Ok(NativeStatus {
            state: 2,
            sprite_id: 9,
            width: 8,
            height: 11,
            origin_x: -1,
            origin_y: 4,
        }));
        let g = p.get_glyph(k).unwrap();
        assert_eq!((g.left, g.top, g.width, g.height), (-1, 11, 8, 11));
        assert!(matches!(
            g.buffer,
            BitmapBuffer::Native(NativeGlyph { window: 42, sprite: 9, ticket: 17 })
        ));
        assert!(!p.take_redraw_request());
    }
    #[test]
    fn busy_retries_and_terminal_errors_stop_redraw() {
        let (mut p, k) = setup();
        p.host.busy.set(true);
        assert!(matches!(p.get_glyph(k), Err(Error::Pending)));
        assert!(p.take_redraw_request());
        p.host.busy.set(false);
        p.host.status.set(Err(-5));
        assert!(matches!(p.get_glyph(k), Err(Error::PlatformError(_))));
        assert!(!p.take_redraw_request());
        p.host.status.set(Ok(NativeStatus { state: 3, ..Default::default() }));
        assert!(matches!(p.get_glyph(k), Err(Error::MissingGlyph(_))));
        assert!(!p.take_redraw_request());
    }
    #[test]
    fn persistent_capacity_pressure_stops_retrying() {
        let (mut p, k) = setup();
        p.host.busy.set(true);
        for _ in 0..119 {
            assert!(matches!(p.get_glyph(k), Err(Error::Pending)));
            assert!(p.take_redraw_request());
        }
        assert!(matches!(p.get_glyph(k), Err(Error::PlatformError(_))));
        assert!(!p.take_redraw_request());
        assert!(p.pending.is_empty() && p.busy.is_empty());
    }
    #[test]
    fn whitespace_needs_no_sprite_and_unsupported_style_is_explicit() {
        let (mut p, k) = setup();
        assert_eq!(p.get_glyph(GlyphKey { character: ' ', ..k }).unwrap().width, 0);
        assert_eq!(p.host.requests.get(), 0);
        let bold = FontDesc::new(
            "monospace",
            Style::Description { slant: Slant::Normal, weight: Weight::Bold },
        );
        assert!(matches!(p.load_font(&bold, k.size), Err(Error::FontNotFound(_))));
    }
}
