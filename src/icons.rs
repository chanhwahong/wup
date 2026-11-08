//! Small monochrome vector icons, painted locally without font or asset files.
use gpui::{IntoElement, PathBuilder, Styled, canvas, point, px, rems, rgb};

#[derive(Clone, Copy)]
pub enum Icon {
    Refresh,
    Info,
    Mail,
    Link,
    Coffee,
    Square,
    CheckSquare,
    Packages,
    Trash,
    Upgrade,
    Log,
    Copy,
    Stop,
    Close,
    Theme,
    Language,
    ChevronDown,
    ChevronLeft,
    More,
    Scale,
}

pub fn icon(name: Icon, color: u32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let scale = f32::from(bounds.size.width) / 16.;
            let mut path = PathBuilder::stroke(px(1.4 * scale));
            let mut line = |points: &[(f32, f32)]| {
                for (i, &(x, y)) in points.iter().enumerate() {
                    let p = point(
                        bounds.origin.x + px(x * f32::from(bounds.size.width) / 24.),
                        bounds.origin.y + px(y * f32::from(bounds.size.height) / 24.),
                    );
                    if i == 0 {
                        path.move_to(p);
                    } else {
                        path.line_to(p);
                    }
                }
            };
            match name {
                Icon::Info => {
                    let circle: Vec<_> = (0..=32)
                        .map(|i| {
                            let a = i as f32 * std::f32::consts::TAU / 32.;
                            (12. + 9. * a.cos(), 12. + 9. * a.sin())
                        })
                        .collect();
                    line(&circle);
                    line(&[(12., 10.), (12., 17.)]);
                    line(&[(12., 6.5), (12., 7.5)]);
                }
                Icon::Mail => {
                    line(&[(3., 5.), (21., 5.), (21., 19.), (3., 19.), (3., 5.)]);
                    line(&[(3., 6.), (12., 13.), (21., 6.)]);
                }
                Icon::Link => {
                    line(&[(9., 8.), (9., 5.), (20., 5.), (20., 16.), (17., 16.)]);
                    line(&[(14., 11.), (14., 20.), (3., 20.), (3., 9.), (12., 9.)]);
                    line(&[(10., 15.), (20., 5.)]);
                }
                Icon::Coffee => {
                    line(&[
                        (4., 8.),
                        (17., 8.),
                        (17., 16.),
                        (14., 19.),
                        (7., 19.),
                        (4., 16.),
                        (4., 8.),
                    ]);
                    line(&[(17., 9.), (21., 9.), (22., 11.), (21., 14.), (17., 14.)]);
                    line(&[(3., 22.), (19., 22.)]);
                    line(&[(8., 2.), (8., 5.)]);
                    line(&[(13., 2.), (13., 5.)]);
                }
                Icon::ChevronLeft => line(&[(14., 6.), (8., 12.), (14., 18.)]),
                Icon::More => {
                    for x in [5., 12., 19.] {
                        let dot: Vec<_> = (0..=8)
                            .map(|i| {
                                let a = i as f32 * std::f32::consts::TAU / 8.;
                                (x + a.cos(), 12. + a.sin())
                            })
                            .collect();
                        line(&dot);
                    }
                }
                Icon::Scale => {
                    line(&[(4., 9.), (4., 4.), (9., 4.)]);
                    line(&[(15., 4.), (20., 4.), (20., 9.)]);
                    line(&[(20., 15.), (20., 20.), (15., 20.)]);
                    line(&[(9., 20.), (4., 20.), (4., 15.)]);
                    line(&[(8., 12.), (16., 12.)]);
                    line(&[(12., 8.), (12., 16.)]);
                }
                Icon::ChevronDown => line(&[(6., 9.), (12., 15.), (18., 9.)]),
                Icon::Refresh => {
                    let arc: Vec<_> = (0..=28)
                        .map(|i| {
                            let angle = (-45. + i as f32 * 285. / 28.).to_radians();
                            (12. + 8. * angle.cos(), 12. + 8. * angle.sin())
                        })
                        .collect();
                    line(&arc);
                    line(&[(18., 2.), (18., 7.), (23., 7.)]);
                }
                Icon::Square | Icon::CheckSquare => {
                    line(&[(4., 4.), (20., 4.), (20., 20.), (4., 20.), (4., 4.)]);
                    if matches!(name, Icon::CheckSquare) {
                        line(&[(7., 12.), (10.5, 15.5), (17., 9.)]);
                    }
                }
                Icon::Packages => {
                    line(&[(3., 6.), (7., 6.), (7., 10.), (3., 10.), (3., 6.)]);
                    line(&[(3., 14.), (7., 14.), (7., 18.), (3., 18.), (3., 14.)]);
                    line(&[(11., 8.), (21., 8.)]);
                    line(&[(11., 16.), (21., 16.)]);
                }
                Icon::Trash => {
                    line(&[(3., 6.), (21., 6.)]);
                    line(&[(8., 6.), (8., 3.), (16., 3.), (16., 6.)]);
                    line(&[(6., 6.), (7., 21.), (17., 21.), (18., 6.)]);
                    line(&[(10., 9.), (10., 17.)]);
                    line(&[(14., 9.), (14., 17.)]);
                }
                Icon::Upgrade => {
                    line(&[(12., 17.), (12., 3.)]);
                    line(&[(6., 9.), (12., 3.), (18., 9.)]);
                    line(&[(4., 15.), (4., 21.), (20., 21.), (20., 15.)]);
                }
                Icon::Log => {
                    line(&[(4., 4.), (20., 4.), (20., 20.), (4., 20.), (4., 4.)]);
                    line(&[(7., 8.), (10., 11.), (7., 14.)]);
                    line(&[(12., 15.), (17., 15.)]);
                }
                Icon::Copy => {
                    line(&[(8., 8.), (21., 8.), (21., 21.), (8., 21.), (8., 8.)]);
                    line(&[(16., 5.), (16., 3.), (3., 3.), (3., 16.), (5., 16.)]);
                }
                Icon::Stop => line(&[(5., 5.), (19., 5.), (19., 19.), (5., 19.), (5., 5.)]),
                Icon::Close => {
                    line(&[(5., 5.), (19., 19.)]);
                    line(&[(19., 5.), (5., 19.)]);
                }
                Icon::Theme => {
                    line(&[(3., 6.), (21., 6.)]);
                    line(&[(3., 12.), (21., 12.)]);
                    line(&[(3., 18.), (21., 18.)]);
                    line(&[(8., 3.), (8., 9.)]);
                    line(&[(16., 9.), (16., 15.)]);
                    line(&[(10., 15.), (10., 21.)]);
                }
                Icon::Language => {
                    let circle: Vec<_> = (0..=24)
                        .map(|i| {
                            let a = i as f32 * std::f32::consts::TAU / 24.;
                            (12. + 9. * a.cos(), 12. + 9. * a.sin())
                        })
                        .collect();
                    line(&circle);
                    line(&[(3., 12.), (21., 12.)]);
                    line(&[(12., 3.), (8., 7.), (7., 12.), (8., 17.), (12., 21.)]);
                    line(&[(12., 3.), (16., 7.), (17., 12.), (16., 17.), (12., 21.)]);
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(color));
            }
        },
    )
    .size(rems(1.))
    .flex_shrink_0()
}

/// A looping progress ring. GPUI schedules frames only while it is mounted.
pub fn spinner(id: impl Into<gpui::ElementId>, color: u32) -> impl IntoElement {
    use gpui::{Animation, AnimationExt, ParentElement, div};
    use std::time::Duration;

    div().size(rems(1.)).flex_shrink_0().with_animation(
        id,
        Animation::new(Duration::from_millis(900)).repeat(),
        move |element, phase| {
            element.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let diameter = f32::from(bounds.size.width);
                        let center = bounds.center();
                        let radius = diameter * 0.38;
                        let mut path = PathBuilder::stroke(px(diameter * 0.12));
                        for step in 0..=48 {
                            let angle = (phase + step as f32 / 64.) * std::f32::consts::TAU;
                            let position = point(
                                center.x + px(radius * angle.cos()),
                                center.y + px(radius * angle.sin()),
                            );
                            if step == 0 {
                                path.move_to(position);
                            } else {
                                path.line_to(position);
                            }
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, rgb(color));
                        }
                    },
                )
                .size_full(),
            )
        },
    )
}
