use eframe::egui::ViewportBuilder;
use nimble::gui::NimbleGui;
use nimble::gui::state::GuiConfig;

fn main() -> Result<(), eframe::Error> {
    let config = GuiConfig::load();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(config.window_size()),
        ..Default::default()
    };
    
    eframe::run_native(
        "Nimble",
        options,
        Box::new(|cc| Ok(Box::new(NimbleGui::new(cc))))
    )
}
