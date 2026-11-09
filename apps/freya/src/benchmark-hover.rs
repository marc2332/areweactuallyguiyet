use std::time::Duration;

use benchmark_common::{HoverObserver, Scenario, hover};
use freya::prelude::*;

use crate::ui::{self, HarnessContext};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let context = HarnessContext::connect(Scenario::Hover)?;
    let failure = context.failure.clone();
    ui::launch(app, context, failure)
}

fn app() -> Element {
    let context = use_hook(consume_root_context::<HarnessContext>);
    use_hook(|| {
        provide_root_context(context.harness.borrow().hover_observer());
    });
    let platform = Platform::get();
    use_future(move || {
        let context = context.clone();
        let platform = platform.clone();
        async move {
            loop {
                async_io::Timer::after(Duration::from_millis(16)).await;
                match context.harness.borrow_mut().poll() {
                    Ok(frame) if frame.should_quit => break,
                    Ok(_) => {}
                    Err(error) => {
                        *context.failure.borrow_mut() = Some(error.to_string());
                        break;
                    }
                }
            }
            platform.exit();
        }
    });
    HoverGrid.into()
}

#[derive(PartialEq)]
struct HoverGrid;

impl Component for HoverGrid {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(hover::BACKGROUND)
            .children((0..hover::ROWS).map(|row| {
                rect()
                    .key(row)
                    .horizontal()
                    .width(Size::px(hover::GRID_SIZE))
                    .height(Size::px(hover::CELL_SIZE))
                    .children((0..hover::COLUMNS).map(move |column| HoverCell {
                        index: row * hover::COLUMNS + column,
                    }))
            }))
    }
}

#[derive(PartialEq)]
struct HoverCell {
    index: usize,
}

impl Component for HoverCell {
    fn render(&self) -> impl IntoElement {
        let index = self.index;
        let mut hovered = use_state(|| false);
        let observer = use_hook(consume_root_context::<HoverObserver>);
        let leave_observer = observer.clone();
        rect()
            .width(Size::px(hover::CELL_SIZE))
            .height(Size::px(hover::CELL_SIZE))
            .background(if *hovered.read() {
                hover::HOVERED_COLOR
            } else {
                hover::NORMAL_COLOR
            })
            .on_pointer_enter(move |_| {
                hovered.set_if_modified(true);
                observer.enter(index);
            })
            .on_pointer_leave(move |_| {
                hovered.set_if_modified(false);
                leave_observer.leave(index);
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::HoverGrid;
    use benchmark_common::{HoverObserver, hover};
    use freya::prelude::*;
    use freya_testing::TestingRunner;

    fn grid() -> Element {
        HoverGrid.into()
    }

    #[test]
    fn fullscreen_hover_grid_fills_viewport_and_keeps_cells_at_origin_on_enter_and_leave() {
        let (mut runner, _) = TestingRunner::new(
            grid,
            (1920., 1080.).into(),
            |runner| runner.provide_root_context(HoverObserver::default),
            1.,
        );
        runner.sync_and_update();
        let backgrounds = runner.find_many(|node, element| {
            let rectangle = Rect::try_downcast(element)?;
            let area = node.layout().area;
            (rectangle.style.background == Color::from(hover::BACKGROUND).into()).then_some(area)
        });
        assert_eq!(backgrounds.len(), 1);
        assert_eq!(backgrounds[0].origin.x, 0.);
        assert_eq!(backgrounds[0].origin.y, 0.);
        assert_eq!(backgrounds[0].size.width, 1920.);
        assert_eq!(backgrounds[0].size.height, 1080.);
        assert_cells(&runner, None);
        runner.move_cursor((12., 12.));
        assert_cells(&runner, Some(0));
        runner.move_cursor((36., 12.));
        assert_cells(&runner, Some(1));
        runner.move_cursor((708., 708.));
        assert_cells(&runner, Some(899));
        runner.move_cursor((372., 372.));
        assert_cells(&runner, Some(465));
        runner.move_cursor((740., 740.));
        assert_cells(&runner, None);
    }

    fn assert_cells(runner: &TestingRunner, hovered_index: Option<usize>) {
        let colors = runner.find_many(|node, element| {
            let rectangle = Rect::try_downcast(element)?;
            let area = node.layout().area;
            if area.size.width != hover::CELL_SIZE || area.size.height != hover::CELL_SIZE {
                return None;
            }
            Some((area.origin, rectangle.style.background.clone()))
        });
        assert_eq!(colors.len(), hover::COUNT);
        for (index, (origin, background)) in colors.into_iter().enumerate() {
            assert_eq!(origin.x, (index % hover::COLUMNS) as f32 * hover::CELL_SIZE);
            assert_eq!(origin.y, (index / hover::COLUMNS) as f32 * hover::CELL_SIZE);
            let expected = if Some(index) == hovered_index {
                hover::HOVERED_COLOR
            } else {
                hover::NORMAL_COLOR
            };
            assert_eq!(background, Color::from(expected).into());
        }
    }
}
