use benchmark_common::{HoverObserver, Scenario, hover};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<HoverGrid>()
}

struct HoverGrid;

impl crate::ui::Adapter for HoverGrid {
    type State = ();
    const SCENARIO: Scenario = Scenario::Hover;

    fn state() {}

    fn build(cx: &mut Context, _: &(), observer: HoverObserver) {
        VStack::new(cx, |cx| {
            for row in 0..hover::ROWS {
                HStack::new(cx, |cx| {
                    for column in 0..hover::COLUMNS {
                        HoverCell {
                            index: row * hover::COLUMNS + column,
                            observer: observer.clone(),
                        }
                        .build(cx, |_| {})
                        .width(Pixels(hover::CELL_SIZE))
                        .height(Pixels(hover::CELL_SIZE))
                        .background_color(color(hover::NORMAL_COLOR));
                    }
                })
                .width(Pixels(hover::GRID_SIZE))
                .height(Pixels(hover::CELL_SIZE))
                .gap(Pixels(0.0))
                .padding(Pixels(0.0));
            }
        })
        .width(Pixels(hover::GRID_SIZE))
        .height(Pixels(hover::GRID_SIZE))
        .gap(Pixels(0.0))
        .padding(Pixels(0.0));
    }
}

struct HoverCell {
    index: usize,
    observer: HoverObserver,
}

impl HoverCell {
    fn native_event(&self, event: &WindowEvent) -> Option<Color> {
        match event {
            WindowEvent::MouseEnter => {
                self.observer.enter(self.index);
                Some(color(hover::HOVERED_COLOR))
            }
            WindowEvent::MouseLeave => {
                self.observer.leave(self.index);
                Some(color(hover::NORMAL_COLOR))
            }
            _ => None,
        }
    }
}

impl View for HoverCell {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|event: &WindowEvent, _| {
            if let Some(color) = self.native_event(event) {
                cx.set_background_color(color);
            }
        });
    }
}

fn color(rgb: (u8, u8, u8)) -> Color {
    Color::rgb(rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::{HoverCell, color};
    use benchmark_common::{HoverObserver, hover};
    use vizia::prelude::WindowEvent;

    #[test]
    fn hover_cell_uses_only_native_targeted_enter_and_leave_events() {
        let cell = HoverCell {
            index: 899,
            observer: HoverObserver::default(),
        };
        assert_eq!(
            cell.native_event(&WindowEvent::MouseEnter),
            Some(color(hover::HOVERED_COLOR))
        );
        assert_eq!(
            cell.native_event(&WindowEvent::MouseLeave),
            Some(color(hover::NORMAL_COLOR))
        );
        assert_eq!(cell.native_event(&WindowEvent::MouseOver), None);
        assert_eq!(cell.native_event(&WindowEvent::MouseOut), None);
        assert_eq!(cell.native_event(&WindowEvent::MouseMove(12.0, 12.0)), None);
    }
}
