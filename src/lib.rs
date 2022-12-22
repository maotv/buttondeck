mod deck;
mod button;
mod error;
mod device;
mod setup;
mod hardware;

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
pub use button::ButtonRef;
pub use button::StateRef2;
pub use button::ButtonColor;
pub use button::ButtonState;
