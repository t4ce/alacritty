//! TRUEOS GL profile 0 program manifest.
//!
//! The matching Bakery publication owns the native vertex/fragment artifacts.
//! These stable virtual names are the only shader-program handles exposed to
//! the Alacritty GLES2Pure renderer; they are not general OpenGL program IDs.

use std::ffi::CStr;

use crate::gl::types::{GLint, GLuint};

const PROGRAM_BASE: GLuint = 0x5452_0000; // "TR"; reserved for TRUEOS AOT GL.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgramId {
    TextPure = 0,
    RectNormal = 1,
    RectUndercurl = 2,
    RectDotted = 3,
    RectDashed = 4,
}

impl ProgramId {
    pub(crate) const fn gl_name(self) -> GLuint {
        PROGRAM_BASE + self as GLuint + 1
    }

    pub(crate) const fn from_gl_name(name: GLuint) -> Option<Self> {
        match name.checked_sub(PROGRAM_BASE + 1) {
            Some(0) => Some(Self::TextPure),
            Some(1) => Some(Self::RectNormal),
            Some(2) => Some(Self::RectUndercurl),
            Some(3) => Some(Self::RectDotted),
            Some(4) => Some(Self::RectDashed),
            _ => None,
        }
    }

    /// The Bakery manifest's compact reflection table.
    pub(crate) fn uniform_location(self, name: &CStr) -> Option<GLint> {
        let name = name.to_bytes();
        match self {
            Self::TextPure => match name {
                b"projection" => Some(0),
                b"renderingPass" => Some(1),
                b"mask" => Some(2),
                _ => None,
            },
            Self::RectNormal | Self::RectUndercurl | Self::RectDotted | Self::RectDashed => {
                match name {
                    b"cellWidth" => Some(0),
                    b"cellHeight" => Some(1),
                    b"paddingY" => Some(2),
                    b"paddingX" => Some(3),
                    b"underlinePosition" => Some(4),
                    b"underlineThickness" => Some(5),
                    b"undercurlPosition" => Some(6),
                    _ => None,
                }
            },
        }
    }
}
