mod deck;
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
pub use deck::Button;
pub use deck::ButtonFn;
pub use deck::BtnRef;
pub use deck::DeckEvent;
pub use deck::ButtonSetup;
pub use deck::ButtonColor;
pub use deck::ButtonState;
