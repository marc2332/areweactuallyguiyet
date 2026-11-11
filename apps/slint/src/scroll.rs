#[path = "../../scroll_config.rs"]
mod scroll_config;

slint::slint! {
    import { Palette, ScrollView } from "std-widgets.slint";

    export component ScrollWindow inherits Window {
        title: "Slint Scroll";
        in property <int> row-count;
        width: 1000px;
        height: 700px;
        background: white;
        init => { Palette.color-scheme = ColorScheme.light; }

        scroll := ScrollView {
            width: parent.width;
            height: parent.height;
            content-width: 960px;
            content-height: root.row-count * 48px;
            horizontal-scrollbar-policy: always-off;

            for index in root.row-count: Rectangle {
                y: index * 48px;
                width: 960px;
                height: 48px;
                background: mod(index, 2) == 0 ? #f5f5f5 : #e6e6e6;

                Text {
                    x: 12px;
                    y: 12px;
                    width: parent.width - 24px;
                    height: parent.height - 24px;
                    text: "Item " + (index + 1);
                    color: black;
                    font-size: 14px;
                    vertical-alignment: center;
                }
            }
        }
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    let window = ScrollWindow::new()?;
    window.set_row_count(scroll_config::row_count() as i32);
    window.run()
}
