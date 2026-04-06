mod app;
mod audio;
mod ui;

use app::App;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut app = App::default();
    ratatui::run(|terminal| app.run(terminal))
}
