
#[derive(Debug)]
pub enum DeviceFamily {
    Midi,
    Streamdeck
}

impl Default for DeviceFamily {
    fn default() -> Self {
        DeviceFamily::Streamdeck
    }
}

#[derive(Clone,Copy)]
pub enum DeviceKind {
    GenericMidi,
    AkaiFire,
    TouchOSC,
    KorgNanoKontrol2,
    StreamDeck,
    StreamDeckOriginal,
    StreamDeckOriginalV2,
    StreamDeckMini,
    StreamDeckXL,
    StreamDeckMK2,
}



impl Default for DeviceKind {
    fn default() -> Self {
        DeviceKind::StreamDeck
    }
}

impl DeviceKind {
    pub fn get_specs(&self) -> DeviceSpecs {
        match self {
            DeviceKind::GenericMidi => DeviceSpecs { 
                family: DeviceFamily::Midi,
                ..Default::default()
            },
            DeviceKind::StreamDeck  => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
            DeviceKind::AkaiFire    => DeviceSpecs { 
                family: DeviceFamily::Midi,
                ..Default::default()
            },
            DeviceKind::TouchOSC => DeviceSpecs { 
                family: DeviceFamily::Midi,
                ..Default::default()
            },
            DeviceKind::KorgNanoKontrol2 => DeviceSpecs { 
                family: DeviceFamily::Midi,
                ..Default::default()
            },
            DeviceKind::StreamDeckMini => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
            DeviceKind::StreamDeckOriginal => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
            DeviceKind::StreamDeckOriginalV2 => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
            DeviceKind::StreamDeckXL => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
            DeviceKind::StreamDeckMK2 => DeviceSpecs { 
                family: DeviceFamily::Streamdeck,
                ..Default::default()
            },
        }
    }
}


#[derive(Debug,Default)]
pub struct DeviceSpecs {
    pub family: DeviceFamily,
    pub midi_in: Option<String>,
    pub midi_out: Option<String>,
}


pub enum ButtonClass {
    MidiCcButton,
    MidiCcValue,
    MidiNote,

}


pub fn discover() {

}