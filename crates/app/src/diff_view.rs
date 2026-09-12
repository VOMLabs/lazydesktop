//! Diff view — placeholder for the file diff viewer.

use gpui::*;
use gpui_component::*;

pub struct DiffView;

impl DiffView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .p_4()
            .child(div().text_lg().font_bold().child("Diff View"))
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(0x8c959f))
                    .child("Select a file to view its diff"),
            )
    }
}
