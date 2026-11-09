use benchmark_common::{
    Harness, Scenario,
    hover::{self, HoverObserver},
};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{Space, column, container, mouse_area, row},
};
use std::{cell::Cell, time::Duration};

struct App {
    harness: Harness,
    observer: HoverObserver,
    readiness_requested: bool,
    ready_sent: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    InitialFrame,
    Ready(
        Result<&'static str, String>,
        f32,
        iced::Size,
        iced::window::Mode,
        Option<iced::Size>,
    ),
    HoverObserved,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::Hover).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        let observer = harness.hover_observer();
        (
            Self {
                harness,
                observer,
                readiness_requested: false,
                ready_sent: false,
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(Duration::from_millis(33)).map(|_| Message::Tick),
            if self.readiness_requested {
                Subscription::none()
            } else {
                iced::window::frames().map(|_| Message::InitialFrame)
            },
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InitialFrame if !self.readiness_requested => {
                self.readiness_requested = true;
                return iced::window::oldest().then(|window| match window {
                    Some(window) => iced::window::run(window, |window| {
                        use iced::window::raw_window_handle::RawDisplayHandle;
                        Ok(
                            match window.display_handle().ok().map(|handle| handle.as_raw()) {
                                Some(RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_)) => "x11",
                                Some(RawDisplayHandle::Wayland(_)) => "wayland",
                                _ => "unspecified",
                            },
                        )
                    })
                    .then(move |backend| {
                        iced::window::mode(window).then(move |mode| {
                            let backend = backend.clone();
                            iced::window::monitor_size(window).then(move |monitor| {
                                let backend = backend.clone();
                                iced::window::scale_factor(window).then(move |scale| {
                                    let backend = backend.clone();
                                    iced::window::size(window).map(move |size| {
                                        Message::Ready(backend.clone(), scale, size, mode, monitor)
                                    })
                                })
                            })
                        })
                    }),
                    None => Task::done(Message::Ready(
                        Err("Iced benchmark window is unavailable".to_owned()),
                        0.0,
                        iced::Size::ZERO,
                        iced::window::Mode::Windowed,
                        None,
                    )),
                });
            }
            Message::Ready(backend, scale, size, mode, monitor) if !self.ready_sent => {
                if backend.is_ok() && !fullscreen_viewport_ready(mode, size, monitor) {
                    self.readiness_requested = false;
                    return Task::none();
                }
                let result = backend.and_then(|backend| {
                    self.harness
                        .mark_hover_ready(
                            "initial-draw-returned-not-presented",
                            backend,
                            f64::from(scale),
                            size.width,
                            size.height,
                        )
                        .map_err(|error| error.to_string())
                });
                if let Err(error) = result {
                    eprintln!("Iced benchmark readiness failed: {error}");
                    std::process::exit(2);
                }
                self.ready_sent = true;
            }
            Message::Tick => match self.harness.poll() {
                Ok(state) if state.should_quit => return iced::exit(),
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Iced benchmark protocol failed: {error}");
                    std::process::exit(2);
                }
            },
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let mut grid = column![].spacing(0);
        for row_index in 0..hover::ROWS {
            let mut line = row![].spacing(0);
            for column in 0..hover::COLUMNS {
                line = line.push(Element::new(HoverCell::new(
                    row_index * hover::COLUMNS + column,
                    self.observer.clone(),
                )));
            }
            grid = grid.push(line);
        }
        container(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(color(hover::BACKGROUND).into()),
                ..Default::default()
            })
            .into()
    }
}

fn fullscreen_viewport_ready(
    mode: iced::window::Mode,
    size: iced::Size,
    monitor: Option<iced::Size>,
) -> bool {
    mode == iced::window::Mode::Fullscreen
        && size.width > 0.0
        && size.height > 0.0
        && monitor == Some(size)
}

#[derive(Clone)]
enum CellEvent {
    Enter,
    Leave,
}

struct HoverCell {
    index: usize,
    observer: HoverObserver,
    target: Element<'static, CellEvent>,
}

impl HoverCell {
    fn new(index: usize, observer: HoverObserver) -> Self {
        Self {
            index,
            observer,
            target: mouse_area(
                Space::new()
                    .width(hover::CELL_SIZE)
                    .height(hover::CELL_SIZE),
            )
            .on_enter(CellEvent::Enter)
            .on_exit(CellEvent::Leave)
            .into(),
        }
    }
}

impl Widget<Message, iced::Theme, iced::Renderer> for HoverCell {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Cell<bool>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Cell::new(false))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.target)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.target));
    }

    fn size(&self) -> iced::Size<Length> {
        self.target.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.target
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        let mut events = Vec::new();
        let mut native_shell = Shell::new(&mut events);
        self.target.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut native_shell,
            viewport,
        );
        if !native_shell.is_empty() {
            native_shell.request_redraw();
        }
        let hovered = tree.state.downcast_ref::<Cell<bool>>();
        shell.merge(native_shell, |event| {
            match event {
                CellEvent::Enter => {
                    hovered.set(true);
                    self.observer.enter(self.index);
                }
                CellEvent::Leave => {
                    hovered.set(false);
                    self.observer.leave(self.index);
                }
            }
            Message::HoverObserved
        });
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        use iced::advanced::Renderer;
        let hovered = tree.state.downcast_ref::<Cell<bool>>().get();
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..Default::default()
            },
            color(if hovered {
                hover::HOVERED_COLOR
            } else {
                hover::NORMAL_COLOR
            }),
        );
    }
}

fn color(value: (u8, u8, u8)) -> iced::Color {
    iced::Color::from_rgb8(value.0, value.1, value.2)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: hover".into(),
        App::subscription,
    )
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::fullscreen_viewport_ready;
    use iced::{Size, window::Mode};

    #[test]
    fn fullscreen_readiness_rejects_transitional_size_and_requires_native_mode() {
        let output = Size::new(1920., 1080.);
        assert!(fullscreen_viewport_ready(
            Mode::Fullscreen,
            output,
            Some(output)
        ));
        assert!(!fullscreen_viewport_ready(
            Mode::Windowed,
            output,
            Some(output)
        ));
        assert!(!fullscreen_viewport_ready(
            Mode::Fullscreen,
            Size::new(1280., 800.),
            Some(output)
        ));
        assert!(!fullscreen_viewport_ready(Mode::Fullscreen, output, None));
        assert!(!fullscreen_viewport_ready(
            Mode::Fullscreen,
            Size::ZERO,
            Some(Size::ZERO)
        ));
    }
}
