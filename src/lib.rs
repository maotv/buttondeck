
use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell; 
use std::rc::Rc;
use std::path::{PathBuf, Path}; 
use std::str::FromStr;
use std::time::Duration; 
use std::fs::File;

use hidapi::HidApi;
use log::{warn, debug, error};

use serde_derive::{Serialize, Deserialize};
use thiserror::Error;

use self::device::{StreamDeckDevice};




mod deck;
mod error;
mod device;
mod setup;

pub use error::DeckError;

pub use device::ButtonDevice;

pub use setup::ButtonDeckBuilder;

pub use deck::ButtonDeck;
pub use deck::Button;
pub use deck::ButtonFn;
pub use deck::BtnRef;
pub use deck::DeckEvent;
pub use deck::ButtonSetup;
pub use deck::ButtonColor;
pub use deck::ButtonState;

type Result<T> = std::result::Result<T,DeckError>;







// fn get_button_down_fn(btn: Option<&Button>) -> Option<ButtonFn> {

//     match btn {
//         Some(b) => b.on_button_down,
//         None => None
//     }

// }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        
    }
}
