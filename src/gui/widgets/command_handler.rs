use std::sync::mpsc::Sender;
use crate::gui::state::CommandMessage;

pub trait CommandHandler {
    fn try_send(&self, sender: Option<&Sender<CommandMessage>>, msg: CommandMessage) {
        if let Some(sender) = sender {
            sender.send(msg).ok();
        }
    }

    fn handle_validation<F>(
        validation: impl FnOnce() -> Result<(), String>,
        error_handler: impl FnOnce(String),
        success_handler: F,
        sender: Option<&Sender<CommandMessage>>)
    where F: FnOnce(Option<&Sender<CommandMessage>>)
    {
        match validation() {
            Ok(()) => success_handler(sender),
            Err(e) => error_handler(e),
        }
    }
}
