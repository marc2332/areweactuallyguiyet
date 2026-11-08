use std::{cell::RefCell, rc::Rc};

use benchmark_common::{Harness, Scenario};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{row_body, row_title};
use freya::prelude::launch as freya_launch;
use freya::prelude::*;

#[derive(Clone)]
pub struct HarnessContext {
    pub harness: Rc<RefCell<Harness>>,
    pub failure: Rc<RefCell<Option<String>>>,
}

impl HarnessContext {
    pub fn connect(scenario: Scenario) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            harness: Rc::new(RefCell::new(Harness::connect(scenario)?)),
            failure: Rc::new(RefCell::new(None)),
        })
    }
}

pub fn launch<C: Clone + 'static>(
    app: fn() -> Element,
    context: C,
    failure: Rc<RefCell<Option<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    freya_launch(
        LaunchConfig::new()
            .with_default_font("DejaVu Sans")
            .with_font(
                "DejaVu Sans",
                Bytes::from_static(include_bytes!("../../../assets/fonts/DejaVuSans.ttf")),
            )
            .with_window(WindowConfig::new(app).with_root_context(context).with_size(
                f64::from(benchmark_common::WIDTH),
                f64::from(benchmark_common::HEIGHT),
            )),
    );
    match failure.borrow_mut().take() {
        Some(message) => Err(message.into()),
        None => Ok(()),
    }
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row(index: usize) -> Element {
    rect()
        .key(index)
        .height(Size::px(benchmark_common::ROW_HEIGHT))
        .padding(4.)
        .child(label().text(row_title(index)))
        .child(label().text(row_body(index)))
        .into()
}
