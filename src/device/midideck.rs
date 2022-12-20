use std::{sync::mpsc, time::Duration};

use log::error;
use midir::{MidiInput, MidiOutput, Ignore};
use wmidi::{MidiMessage, Channel, Note, Velocity, ControlFunction, ControlValue, ProgramNumber, PitchBend};

use crate::{DeviceKind,ButtonDeviceTrait, DeckError, Button};

use super::{DeviceEvent, ButtonDevice};




type Result<T> = std::result::Result<T,DeckError>;


// wmidi MidiMessage has a Lifetime Specifier, so we can not send it over a 
// mpsc::channel, therefore here is a sendable selection of midi messages 
// copied from wmidi 

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendMidi {

    /// This message is sent when a note is released (ended).
    NoteOff(Channel, Note, Velocity),

    /// This message is sent when a note is depressed (start).
    NoteOn(Channel, Note, Velocity),

    /// This message is most often sent by pressing down on the key after it "bottoms out".
    PolyphonicKeyPressure(Channel, Note, Velocity),

    /// This message is sent when a controller value changes. Controllers include devices such as pedals and levers.
    ///
    /// Controller numbers 120-127 are reserved as "Channel Mode Messages".
    ControlChange(Channel, ControlFunction, ControlValue),

    /// This message is sent when the patch number changes.
    ProgramChange(Channel, ProgramNumber),

    /// This message is most often sent by pressing down on the key after it "bottoms out". This message is different
    /// from polyphonic after-touch. Use this message to send the single greatest pressure value (of all the current
    /// depressed keys).
    ChannelPressure(Channel, Velocity),

    /// This message is sent to indicate a change in the pitch bender (wheel or level, typically). The pitch bender is
    /// measured by a fourteen bit value. Center is 8192.
    PitchBendChange(Channel, PitchBend),

    /// Any Other Messsage
    Other(String)

}

impl From<MidiMessage<'_>> for SendMidi {
    fn from(m: MidiMessage) -> Self {
        match m {
            MidiMessage::NoteOff(c, n, v) => SendMidi::NoteOff(c,n,v),
            MidiMessage::NoteOn(c, n, v) => SendMidi::NoteOn(c,n,v),
            MidiMessage::PolyphonicKeyPressure(c, n, v) => SendMidi::PolyphonicKeyPressure(c,n,v),
            MidiMessage::ControlChange(c,f,v) => SendMidi::ControlChange(c,f,v),
/*
            MidiMessage::ProgramChange(_, _) => todo!(),
            MidiMessage::ChannelPressure(_, _) => todo!(),
            MidiMessage::PitchBendChange(_, _) => todo!(),
            MidiMessage::SysEx(_) => todo!(),
            MidiMessage::OwnedSysEx(_) => todo!(),
            MidiMessage::MidiTimeCode(_) => todo!(),
            MidiMessage::SongPositionPointer(_) => todo!(),
            MidiMessage::SongSelect(_) => todo!(),
            MidiMessage::Reserved(_) => todo!(),
            MidiMessage::TuneRequest => todo!(),
            MidiMessage::TimingClock => todo!(),
            MidiMessage::Start => todo!(),
            MidiMessage::Continue => todo!(),
            MidiMessage::Stop => todo!(),
            MidiMessage::ActiveSensing => todo!(),
            MidiMessage::Reset => todo!(),
*/

            _ => SendMidi::Other(format!("{:?}", m))

        }
    }
}


pub struct MidiDevice {
    
    receiver: mpsc::Receiver<SendMidi>,
    // sd: StreamDeck,
    // btn_state: [u8;256],
    // btn_names: [Option<ButtonName>;256],
    model: String,

    midi_out: midir::MidiOutputConnection,
    midi_in: midir::MidiInputConnection<()>,
}

impl MidiDevice {

    fn wait_for_events(&mut self, timeout: usize) -> Result<Vec<super::DeviceEvent>> {

        match self.receiver.recv_timeout(Duration::from_millis(timeout as u64)) {
            Ok(v) => {
                let de = match v {
                    SendMidi::NoteOn(ch,n,v) => Some(DeviceEvent::ButtonDown(u8::from(n) as usize, (u8::from(v) as f32) / 127.0)),
                    SendMidi::NoteOff(ch,n,v) => Some(DeviceEvent::ButtonUp(u8::from(n) as usize)),
                    SendMidi::ControlChange(ch, f, v) => {
                        Some(DeviceEvent::ButtonDown(u8::from(f) as usize, (u8::from(v) as f32) / 127.0))
                    }
                    _ => None
                };
                if let Some(ev) = de {
                    Ok(vec![ev])
                } else {
                    Ok(vec!())
                }
            },
            Err(_e) => Ok(vec!()),
        }


    }

    pub fn start(self, send: mpsc::Sender<DeviceEvent>) -> super::Result<mpsc::Sender<DeviceEvent>> {
        todo!()
    }

}


impl ButtonDeviceTrait for MidiDevice {

    fn model(&self) -> String {
       self.model.clone()
    }


    
}



// println!("MidiMessage: {:?}", mm);
// let de = match mm {

//     MidiMessage::NoteOn(ch,n,v) => Some(DeviceEvent::ButtonDown(u8::from(n) as usize, (u8::from(v) as f32) / 127.0)),
//     MidiMessage::NoteOff(ch,n,v) => Some(DeviceEvent::ButtonUp(u8::from(n) as usize)),
//     MidiMessage::ControlChange(ch, f, v) => {
//         Some(DeviceEvent::ButtonDown(u8::from(f) as usize, (u8::from(v) as f32) / 127.0))
//     }
//     _ => None
// };

// if let Some(ev) = de {
//     if let Err(e) = tx.send(ev) {
//         error!("cannot send device event: {:?}", e);
//     }
// }




pub fn open_midi(device: DeviceKind, ip_name: Option<String>, op_name: Option<String>) -> Result<ButtonDevice> {

    let midi_in   = MidiInput::new("MidiIn")?;
    let midi_out = MidiOutput::new("MidiOut")?;
    
    let in_ports = midi_in.ports();
    let out_ports = midi_out.ports();
    // let first_in = in_ports.iter()
    //     .find(|p| midi_in.port_name(&p).unwrap_or(String::new()).starts_with("FL STUDIO FIRE"));
    
    for ip in &in_ports {
        println!("In-Port: {:?}", midi_in.port_name(&ip))
    }

    for op in &out_ports {
        println!("Out-Port: {:?}", midi_out.port_name(&op))
    }

    let specs = device.get_specs();
    let ipn = ip_name.unwrap_or_else(|| specs.midi_in.unwrap_or_else(|| String::from("TouchOSC")));
    let opn = op_name.unwrap_or_else(|| specs.midi_out.unwrap_or_else(|| String::from("TouchOSC")));

    let in_port = in_ports.into_iter()
        .find(|p| midi_in.port_name(&p).unwrap_or(String::new()) == ipn );

    let out_port = out_ports.into_iter()
        .find(|p| midi_out.port_name(&p).unwrap_or(String::new()) == opn );



    let (tx,rx) = mpsc::channel();
    let tx_move = tx.clone();
        
    // FIXME unwrap
    let mut conn_out = midi_out.connect(&out_port.unwrap(), "midir-test")?;
    
    let mut conn_in  = midi_in.connect(&in_port.unwrap(), "midir-test", move |stamp, message, _| {

        match MidiMessage::try_from(message) {
            Ok(mm) => { // handle_message(stamp, mm),
                println!("MidiMessage: {:?}", mm);
                if let Err(e) = tx.send(SendMidi::from(mm)) {
                    error!("cannot send device event: {:?}", e);
                }
            }
            Err(e) => println!("Error: {}", e)
        }

    }, ())?;



    
    Ok(ButtonDevice::Midi(MidiDevice {
        midi_in: conn_in,
        midi_out: conn_out,
        receiver: rx,
        model: String::from("xxxx"),
    }))



    
    // Err(DeckError::NoDevice)
//     Ok(ButtonDeviceEnum::NoDevice)

}




// pub fn open_fire() -> Result<MidiDevice> {

//     let mut midi_in  = MidiInput::new("Fire Input")?;
//     midi_in.ignore(Ignore::None);

//     let midi_out = MidiOutput::new("Fire Output")?;
   
//     let out_ports = midi_out.ports();
//     let first_out = out_ports.iter()
//         .find(|p| midi_out.port_name(&p).unwrap_or(String::new()).starts_with("FL STUDIO FIRE"));

//     let in_ports = midi_in.ports();
//     let first_in = in_ports.iter()
//         .find(|p| midi_in.port_name(&p).unwrap_or(String::new()).starts_with("FL STUDIO FIRE"));
    
    
//     if first_in.is_none() || first_out.is_none() {
//         return Err(DeckError::Message(String::from("???")));
//     }


//     let (tx,rx) = mpsc::channel();
//     let tx_move = tx.clone();
        
//     let mut conn_out = midi_out.connect(first_out.unwrap(), "midir-test")?;
//     let mut conn_in  = midi_in.connect(first_in.unwrap(), "midir-test", move |stamp, message, _| {

//         match MidiMessage::try_from(message) {
//             Ok(mm) => handle_message(stamp, mm),
//             Err(e) => println!("Error: {}", e)
//         }
        


//         // println!("{:?}: {:?} (len = {})", stamp, message, message.len());
//     }, ())?;



    
//     Ok(MidiDevice {
//         midi_in: conn_in,
//         midi_out: conn_out,
//         receiver: rx,
//         model: String::from("xxxx"),
//     })

// }



// fn handle_message(ts: u64, msg: MidiMessage) {

    
//     match msg {

//         MidiMessage::NoteOn(ch,n,v) => handle_note(true, ch, n, v),
//         MidiMessage::NoteOff(ch,n,v) => handle_note(false, ch, n, v),
//         _ => ()
//     }

// }


// fn handle_note(onoff: bool, ch: Channel, n: Note, v: Velocity) {

// }
