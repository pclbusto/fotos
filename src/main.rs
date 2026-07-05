pub mod app;
pub mod browser;
pub mod codecs;
pub mod dialogs;
pub mod editor;
pub mod message;
pub mod state;
pub mod viewer;

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(900.0)
            .min_height(650.0),
    );
    cosmic::app::run::<app::AppModel>(settings, ())
}
