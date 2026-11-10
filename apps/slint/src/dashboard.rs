slint::slint! {
    import { Button, CheckBox, LineEdit, Palette, ScrollView } from "std-widgets.slint";

    export component DashboardWindow inherits Window {
        title: "Slint Scroll 2000";
        width: 1000px;
        height: 700px;
        background: white;
        init => { Palette.color-scheme = ColorScheme.light; }

        Text { x: 24px; y: 18px; text: "Project dashboard"; color: black; font-size: 24px; }
        LineEdit { x: 24px; y: 70px; width: 360px; height: 40px; text: "Project notes"; }
        property <int> refresh-count: 0;
        Button {
            x: 416px; y: 70px; width: 160px; height: 40px;
            text: "Refresh (" + root.refresh-count + ")";
            clicked => { root.refresh-count += 1; }
        }
        CheckBox { x: 624px; y: 70px; width: 160px; height: 40px; text: "Enabled"; checked: true; }
        Text {
            x: 24px; y: 126px; text: root.refresh-count == 0 ? "Ready" : "Refreshed " + root.refresh-count + " times";
            color: #444444; font-size: 14px;
        }
        ScrollView {
            x: 0px; y: 180px; width: parent.width; height: parent.height - 180px;
            content-width: 960px; content-height: 100 * 112px;
            horizontal-scrollbar-policy: always-off;
            for index in 100: Rectangle {
                x: 20px; y: index * 112px; width: 960px; height: 104px;
                background: mod(index, 2) == 0 ? #f5f5f5 : #e6e6e6;
                Image { x: 12px; y: 12px; width: 64px; height: 64px; source: @image-url("../../../assets/images/dashboard-0.png"); image-fit: contain; }
                Text { x: 92px; y: 16px; text: "Project " + (index + 1); color: #111111; font-size: 17px; }
                Text { x: 92px; y: 44px; width: 820px; text: "A sample project record with notes, ownership details, and recent activity."; color: #555555; font-size: 13px; }
            }
        }
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    DashboardWindow::new()?.run()
}
