slint::slint! {
    import { Palette, ScrollView } from "std-widgets.slint";

    export component ScrollWindow inherits Window {
        title: "Slint Scroll 2000";
        width: 1000px;
        height: 700px;
        background: white;
        init => { Palette.color-scheme = ColorScheme.light; }

        scroll := ScrollView {
            width: parent.width;
            height: parent.height;
            content-width: 960px;
            content-height: 2000 * 48px;
            horizontal-scrollbar-policy: always-off;

            for index in 2000: Rectangle {
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

fn main() -> Result<(), slint::PlatformError> {
    ScrollWindow::new()?.run()
}
