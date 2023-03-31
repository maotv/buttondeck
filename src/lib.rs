mod deck;
mod button;
mod error;
mod device;
mod setup;
mod hardware;
mod sx;

pub use error::DeckError;
pub use device::ButtonDeviceTrait;

pub use hardware::DeviceKind;
pub use hardware::DeviceFamily;
pub use hardware::DeviceSpecs;

pub use setup::ButtonDeckBuilder;

pub use deck::ButtonDeck;
pub use deck::ButtonFn;
pub use deck::FnArg;
pub use deck::DeckEvent;
pub use deck::ButtonSetup;
pub use deck::ButtonDeckSender;

pub use button::Button;
pub use button::ButtonId;
pub use button::StateRef2;
pub use button::ButtonColor;
pub use button::ButtonState;
pub use button::ButtonImage;
pub use button::ButtonValue;


#[macro_export]
macro_rules! elog {
    ($msg:expr, $expression:expr) => {
        if let Err(e) = $expression {
            error!("{}: {}", $msg, e)
        }
    };
    ($expression:expr) => {
        if let Err(e) = $expression {
            error!("{}", e)
        }
    };
}
