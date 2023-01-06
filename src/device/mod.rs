
mod streamdeck;
mod midideck;

use std::path::PathBuf;
use std::sync::mpsc::Sender;

// use crate::ButtonRef;
use crate::ButtonDeck;
use crate::button::ButtonImage;

use super::{DeckError, Button, ButtonColor};

use self::midideck::MidiDevice;
use self::midideck::SendMidi;
pub use self::streamdeck::StreamDeckDevice;
pub use self::streamdeck::open_streamdeck;
pub use self::midideck::open_midi;
pub use self::streamdeck::discover_streamdeck;

type Result<T> = std::result::Result<T,DeckError>;





#[derive(Clone, Debug)]
pub struct PhysicalKey {
    pub id:     usize,
    pub name:   String
}

impl PartialEq for PhysicalKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PhysicalKey {}

pub enum DeviceEventType {
    ButtonDown,
    ButtonUp,
    ButtonValue,
    Aftertouch,
    ControlChange
}
#[derive(Debug)]
pub enum DeviceEvent {
    
    RawMidi(SendMidi),

    ButtonDown(usize,f32),
    ButtonUp(usize),

    SetImage(usize, ButtonImage),
    SetColor(usize, ButtonColor),
    // timestamp: u64,
    // pub kind: DeviceEventType,
    // pub index: usize,
    // pub data:  i32

}



pub enum ButtonDevice {
    Streamdeck(StreamDeckDevice),
    Midi(MidiDevice)
}



pub trait ButtonDeviceTrait {
    // fn start(self, send: Sender<DeviceEvent>) -> Result<Sender<DeviceEvent>>;
    fn model(&self) -> String;
    // fn wait_for_events(&mut self, timeout: usize) -> Result<Vec<DeviceEvent>>;
    // fn decorate_button(&mut self, button: &Button) -> Result<()>;
}