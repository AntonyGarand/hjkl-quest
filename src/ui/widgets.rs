//! Small pieces the screens share: key chips, section labels, overlays.

use nsh_core::prelude::*;

use crate::ui::colors::{LAVENDER, WHITE, hex, hexa};

pub(crate) const MONO: &str = "monospace";

/// A key or a command in a small tinted box.
pub(crate) fn chip(label: impl Into<String>, color: u32) -> impl IntoElement {
    view()
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(hexa(color, 0.16))
        .border_1()
        .border_color(hexa(color, 0.45))
        .child(
            text(label.into())
                .font_family(MONO)
                .text_size(px(13.0))
                .text_color(hex(color))
                .font_weight(FontWeight::BOLD),
        )
}

/// What a key chip does, in a legend of keys.
pub(crate) fn legend(what: &'static str) -> Text {
    text(what).text_color(hexa(WHITE, 0.6))
}

/// A section's title in small spaced capitals.
pub(crate) fn section_label(label: &'static str) -> Text {
    text(label)
        .text_size(px(11.0))
        .letter_spacing(px(1.5))
        .text_color(hexa(LAVENDER, 0.7))
}

/// A layer that covers its parent.
pub(crate) fn cover(element: View) -> View {
    element.absolute().top_0().left_0().right_0().bottom_0()
}

/// A canvas layer that covers its parent.
pub(crate) fn cover_canvas(layer: CanvasView) -> CanvasView {
    layer.absolute().top_0().left_0().right_0().bottom_0()
}
