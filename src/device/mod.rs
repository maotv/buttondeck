
mod streamdeck;
mod midideck;

use std::path::PathBuf;
use std::sync::mpsc::Sender;

// use crate::ButtonRef;
use crate::ButtonDeck;
use crate::DeckEvent;
use crate::button::ButtonImage;

use super::{DeckError, Button, ButtonColor};

use self::midideck::MidiDevice;
use self::midideck::SendMidi;
pub use self::streamdeck::StreamDeckDevice;
// pub use self::streamdeck::open_streamdeck;
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
//    Dummy(DummyDevice),
    Streamdeck(StreamDeckDevice),
    Midi(MidiDevice)
}

impl ButtonDevice {

    pub fn model(&self) -> String {
        self.as_trait().model().clone()
    }

    pub fn as_trait<'a>(&'a self) -> &'a dyn ButtonDeviceTrait {
        let device: &dyn ButtonDeviceTrait = match self {
 //           ButtonDevice::Dummy(d) => d as &dyn ButtonDeviceTrait,
            ButtonDevice::Streamdeck(sd) => sd as &dyn ButtonDeviceTrait,
            ButtonDevice::Midi(md) => md,
        };
        device
    }

    pub fn as_trait_mut<'a>(&'a mut self) -> &'a mut dyn ButtonDeviceTrait {
        let device: &mut dyn ButtonDeviceTrait = match self {
//            ButtonDevice::Dummy(d) => d as &mut dyn ButtonDeviceTrait,
            ButtonDevice::Streamdeck(sd) => sd as &mut dyn ButtonDeviceTrait,
            ButtonDevice::Midi(md) => md,
        };
        device
    }

    pub fn start(self, send: Sender<DeckEvent>) -> Result<Sender<DeviceEvent>> {
        match self {
 //           ButtonDevice::Dummy(d) => d.start(send),
            ButtonDevice::Streamdeck(sd) => sd.start(send),
            ButtonDevice::Midi(md) => md.start(send),
        }
    }
}



pub trait ButtonDeviceTrait {
    fn start(self, send: Sender<DeckEvent>) -> Result<Sender<DeviceEvent>>;
    fn model(&self) -> String;
    // fn wait_for_events(&mut self, timeout: usize) -> Result<Vec<DeviceEvent>>;
    // fn decorate_button(&mut self, button: &Button) -> Result<()>;
}



// pub struct DummyDevice { }

// impl ButtonDeviceTrait for DummyDevice {
//     fn start(self, send: Sender<DeckEvent>) -> Result<Sender<DeviceEvent>> {
//         Err(DeckError::NoDevice)
//     }

//     fn model(&self) -> String {
//         String::from("dummy")
//     }
// }