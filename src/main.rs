pub mod app;
pub mod browser;
pub mod chrome;
pub mod codecs;
pub mod dialog_view;
pub mod dialogs;
pub mod editor;
pub mod footer;
pub mod input;
pub mod message;
pub mod preferences;
pub mod state;
pub mod viewer;

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(900.0)
            .min_height(650.0),
    );
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).map(std::path::PathBuf::from);

    cosmic::app::run::<app::AppModel>(settings, initial_path)
}
