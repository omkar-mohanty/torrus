use iced::{Sandbox, Settings};
use torrus::Result;

struct TorrusApp;

impl Sandbox for TorrusApp {
    type Message = ();

    fn new() -> Self {
        TorrusApp
    }

    fn title(&self) -> String {
        "Torrus".to_string()
    }

    fn update(&mut self, _: Self::Message) {}

    fn view(&self) -> iced::Element<'_, Self::Message> {
        "Torrus".into()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    TorrusApp::run(Settings::default()).unwrap();
    Ok(())
}
