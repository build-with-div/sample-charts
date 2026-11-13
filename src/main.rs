// Release builds open no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use div::prelude::*;
use div::window::WindowSpec;
use sample_charts::ui;

fn main() -> Result<(), Error> {
    App::new("build.div.samples.charts")
        .name("Sample Charts")
        .window(
            WindowSpec::new("main")
                .title("Sample Charts")
                .size(960.0, 640.0)
                .min_size(640.0, 480.0),
            ui::root,
        )
        .run()
}
