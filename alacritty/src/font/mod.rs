//! Platform font boundary: desktop rasterizers or native TRUEOS glyph resources.
#[cfg(not(target_os = "trueos"))]
pub use crossfont::*;
#[cfg(any(target_os = "trueos", test))]
mod trueos;
#[cfg(target_os = "trueos")]
pub use trueos::*;
