use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    pub BenchText = <Label> {
        padding: 0,
        draw_text: {
            color: #111111,
            text_style: {
                font_size: 10.5,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    pub BenchInput = <TextInput> {
        width: Fill, height: 32,
        margin: 0, padding: {left: 6, right: 6, top: 4, bottom: 4},
        draw_text: {
            color: #111111, color_hover: #111111, color_focus: #111111, color_down: #111111,
            color_empty: #555555, color_empty_hover: #555555, color_empty_focus: #555555,
            text_style: {
                font_size: 9,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    pub BenchButton = <Button> {
        margin: 0, padding: 0,
        draw_text: {
            color: #111111, color_hover: #111111, color_focus: #111111, color_down: #111111,
            text_style: {
                font_size: 9,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    pub BenchRow = <View> {
        width: Fill, height: 48, flow: Down,
        padding: {left: 8, right: 8, top: 4, bottom: 4},
        title = <BenchText> {width: Fill, height: 20, draw_text: {wrap: Line}}
        body = <BenchText> {width: Fill, height: 20, draw_text: {wrap: Line, text_style: {font_size: 9}}}
    }
    pub BenchWindow = <Root> {
        main_window = <Window> {
            window: {inner_size: vec2(1280, 800), title: "Makepad benchmark"},
            body = <View> {width: Fill, height: Fill, show_bg: true, draw_bg: {color: #ffffff}}
        }
    }
}
