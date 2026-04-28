use crate::state::AppStore;
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::SingletonEntity as _;
use warpui::{
    elements::{ConstrainedBox, Container, Expanded, Flex, MainAxisSize, ParentElement, Text},
    AppContext, Element, Entity, TypedActionView, View, ViewContext,
};

#[derive(Debug)]
pub enum RootAction {}

pub struct RootView {
    font_family: FamilyId,
    #[allow(dead_code)]
    store: AppStore,
}

impl RootView {
    pub fn new(ctx: &mut ViewContext<Self>, store: AppStore) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());
        Self { font_family, store }
    }

    fn label(&self, text: impl Into<String>, size: f32) -> Box<dyn Element> {
        Text::new(text.into(), self.font_family, size).finish()
    }
}

impl Entity for RootView {
    type Event = ();
}

impl TypedActionView for RootView {
    type Action = RootAction;
}

impl View for RootView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeRoot"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let sidebar = Container::new(
            Flex::column()
                .with_spacing(10.)
                .with_child(self.label("Sessions", 18.))
                .with_child(self.label("New session", 14.))
                .finish(),
        )
        .with_uniform_padding(16.)
        .with_background_color(ColorU::new(24, 28, 36, 255))
        .finish();

        let chat = Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .with_spacing(12.)
                .with_child(self.label("warp-opencode", 22.))
                .with_child(
                    Expanded::new(
                        1.,
                        self.label(
                            "Connects to an OpenCode server and streams agent state via /event.",
                            15.,
                        ),
                    )
                    .finish(),
                )
                .with_child(self.label("Enter to send, Shift+Enter newline", 13.))
                .finish(),
        )
        .with_uniform_padding(18.)
        .with_background_color(ColorU::new(12, 14, 20, 255))
        .finish();

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(ConstrainedBox::new(sidebar).with_width(280.).finish())
            .with_child(Expanded::new(1., chat).finish())
            .finish()
    }
}
