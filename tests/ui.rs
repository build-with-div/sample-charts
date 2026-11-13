//! Headless tests of the window.

use div::prelude::*;
use div::testing::TestApp;
use sample_charts::ui;

#[test]
fn the_window_shows_recharts_axis_ticks() {
    let ui = TestApp::new(LogicalSize::new(960.0, 640.0)).mount(ui::root);
    assert!(ui.find_text("Sample Charts").is_some());
    // getNiceTickValues([0, max], 5) from recharts@3.8.0 under Node.
    for (id, ticks) in [
        ("ticks-bar-default", "0, 80, 160, 240, 320"),
        ("ticks-bar-multiple-mobile", "0, 50, 100, 150, 200"),
        ("ticks-area-interactive-desktop", "0, 150, 300, 450, 600"),
        ("ticks-area-interactive-mobile", "0, 150, 300, 450, 600"),
    ] {
        assert_eq!(ui.by_id(id).text_content().unwrap(), ticks, "{id}");
    }
}
