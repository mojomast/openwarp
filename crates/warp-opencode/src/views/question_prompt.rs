//! Human-in-the-loop question prompt overlay rendering.

use crate::state::AppModel;
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::{
    elements::{
        ChildAnchor, ConstrainedBox, Container, Element, EventDispatchMode, Flex, MainAxisSize,
        OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, Stack, Text,
    },
    prelude::vec2f,
};

pub fn render_question_prompt_overlay(
    base: Box<dyn Element>,
    model: &AppModel,
    font_family: FamilyId,
) -> Box<dyn Element> {
    let Some(question) = model.questions.values().min_by(|a, b| a.id.cmp(&b.id)) else {
        return base;
    };

    let mut body = Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_spacing(10.)
        .with_child(label(
            "Question",
            font_family,
            20.,
            ColorU::new(238, 242, 255, 255),
        ));

    for item in &question.questions {
        let options = item
            .options
            .iter()
            .map(|option| format!("{}: {}", option.label, option.description))
            .collect::<Vec<_>>()
            .join("\n");
        body = body
            .with_child(label(
                &item.question,
                font_family,
                14.,
                ColorU::new(226, 232, 240, 255),
            ))
            .with_child(label(
                options,
                font_family,
                12.,
                ColorU::new(148, 163, 184, 255),
            ));
    }

    let modal = ConstrainedBox::new(
        Container::new(body.finish())
            .with_uniform_padding(20.)
            .with_background_color(ColorU::new(18, 22, 30, 255))
            .finish(),
    )
    .with_width(560.)
    .finish();

    let mut stack = Stack::new().with_event_dispatch_mode(EventDispatchMode::Waterfall);
    stack.add_child(base);
    stack.add_positioned_overlay_child(
        modal,
        OffsetPositioning::offset_from_parent(
            vec2f(0., 0.),
            ParentOffsetBounds::WindowByPosition,
            ParentAnchor::Center,
            ChildAnchor::Center,
        ),
    );
    stack.finish()
}

fn label(
    text: impl Into<String>,
    font_family: FamilyId,
    size: f32,
    color: ColorU,
) -> Box<dyn Element> {
    Text::new(text.into(), font_family, size)
        .with_color(color)
        .finish()
}
