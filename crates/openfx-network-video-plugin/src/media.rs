#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormatKind {
    Rgba,
    Rgbx,
}

pub fn pixel_format_kind(has_alpha: bool) -> PixelFormatKind {
    if has_alpha {
        PixelFormatKind::Rgba
    } else {
        PixelFormatKind::Rgbx
    }
}
