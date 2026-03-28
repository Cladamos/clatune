use std::io;

mod app;
mod audio;
mod ui;

use app::App;

fn main() -> io::Result<()> {
    let mut app = App::default();

    ratatui::run(|terminal| app.run(terminal))
}
