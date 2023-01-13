

use std::{time::{Duration, Instant, SystemTime, UNIX_EPOCH}, str::FromStr, sync::mpsc::{Receiver, Sender, TryRecvError}};
use std::thread;
use std::sync::mpsc;

use hidapi::{DeviceInfo, HidApi};
use log::{error, info, debug, warn};
use streamdeck::{Colour, ImageOptions};
use ::streamdeck::{pids, StreamDeck, Kind};


use crate::{ButtonDeviceTrait, DeviceKind, DeckEvent, elog};

use super::{DeckError, Button, ButtonColor, DeviceEvent, ButtonDevice};


type Result<T> = std::result::Result<T,DeckError>;

const ELGATO: u16 = 0x0fd9;

const BUTTON_OFFSETS: [(Kind,usize); 5] = [
    (Kind::Original, 0),
    (Kind::OriginalV2, 0),
    (Kind::Mini, 1),
    (Kind::Xl, 0),
    (Kind::Mk2, 0),
];


// pub struct MidiDevice {
//     ch: u8,
//     btn_state: [u8;256],
//     model: String

// }





pub struct StreamDeckDevice {
    
    deck: StreamDeck,
    btn_state: [u8;256],
    index_offset: usize,
    // btn_names: [Option<ButtonName>;256],
    model: String

}

impl StreamDeckDevice {

    fn new(mut sd: StreamDeck) -> Self {

        // let model = sd.product().unwrap_or_else(|e| String::from("unknown")).replace(" ","_").to_lowercase();

        let kind = sd.kind();
        let offs = BUTTON_OFFSETS.iter().find(|(k,o)| k == &kind).map(|o| o.1).unwrap_or(0);

        let model = String::from(match &kind {
            Kind::Original => "stream_deck",
            Kind::OriginalV2 => "stream_deck",
            Kind::Mini => "stream_deck_mini",
            Kind::Xl => "stream_deck_xl",
            Kind::Mk2 => "stream_deck",
        });

        StreamDeckDevice {  
            deck: sd,
            btn_state: [0;256],
            index_offset: offs,
            model
        }
    }


}

fn to_colour(c: &ButtonColor) -> Colour {
    Colour {
        r: ((c.rgb&0xff0000) >> 16) as u8,
        g: ((c.rgb&0x00ff00) >> 8) as u8,
        b: (c.rgb&0x0000ff) as u8,
    }
}

impl ButtonDeviceTrait for StreamDeckDevice {
    
    fn model(&self) -> String {
        self.model.clone()
    }


    fn start(self, send_to_buttondeck: Sender<DeckEvent>) -> Result<Sender<DeviceEvent>> {

        let (tx_to_device,rx_from_deck) = mpsc::channel();
        // let txclone = tx_to_device.clone();

        debug!("StreamDeckDevice start");

        thread::spawn(move || {
            readwrite_thread(self, rx_from_deck, send_to_buttondeck);
            error!("readwrite_thread returns");
        });

        Ok(tx_to_device)
    }

}


fn readwrite_thread(mut sd: StreamDeckDevice, rx: Receiver<DeviceEvent>, tx: Sender<DeckEvent>) {

    debug!("readwrite_thread");

    loop {

        let btns = sd.deck.read_buttons(Some(Duration::from_millis(20)));

        match btns {
            Ok(b) => {
                debug!("Btn: {:?}", b);
                for i in 0..b.len() {
                    if sd.btn_state[i] == 0 && b[i] == 1 {
                        elog!(tx.send(DeckEvent::Device(DeviceEvent::ButtonDown(i+sd.index_offset, 1.0)))); 
                    } else  if sd.btn_state[i] == 1 && b[i] == 0 {
                        elog!(tx.send(DeckEvent::Device(DeviceEvent::ButtonUp(i+sd.index_offset))))
                    }
                    sd.btn_state[i] = b[i];
                }
            },
            Err(streamdeck::Error::NoData) => {
                // nothing to do
            },
            Err(e) => {
                error!("Btn Error: {:?}", e);
                elog!(tx.send(DeckEvent::Disconnected));
                std::thread::sleep(Duration::from_millis(1000));
                return;
            }
        }

        loop {
            match rx.try_recv() {
                Ok(DeviceEvent::SetImage(device_index,image)) => {
                    debug!("SetImage");
                    sd.deck.set_button_file((device_index) as u8, &image.path.to_string_lossy(),&ImageOptions::default());
                },
                Ok(DeviceEvent::SetColor(device_index,color)) => {
                    debug!("SetColor");
                    sd.deck.set_button_rgb((device_index) as u8, &to_colour(&color));
                },
                Ok(ev) => {
                    error!("Other event {:?}",ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

//     while rx.try_recv()

}



/* 
fn read_button_events(sd: &mut StreamDeckDevice) -> Result<Vec<DeviceEvent>> {

    let btns = sd.deck.read_buttons(Some(Duration::from_millis(100)));
    let mut events = vec!();

    match btns {
        Ok(b) => {
            debug!("Btn: {:?}", b);
            for i in 0..b.len() {
                if sd.btn_state[i] == 0 && b[i] == 1 {
                    events.push(DeviceEvent::ButtonDown(i, 1.0));
                } else  if sd.btn_state[i] == 1 && b[i] == 0 {
                    events.push(DeviceEvent::ButtonUp(i));
                }
                sd.btn_state[i] = b[i];
            }
        },
        Err(streamdeck::Error::NoData) => {

        },
        Err(e) => {
            error!("Btn Error: {:?}", e);
            return Err(e.into())
        }
    }

    Ok(events)

}
*/



pub fn open_streamdeck(hidapi: &mut HidApi, kind: DeviceKind) -> Result<ButtonDevice> {

    info!("Open Streamdeck");



    if let Err(e) = hidapi.refresh_devices() {
        error!("{:?}",e);
    }

    let alldecks: Vec<u16> = vec![
        pids::ORIGINAL,
        pids::ORIGINAL_V2,
        pids::MINI,
        pids::XL,
        pids::MK2,
    ];

    let devinfo: Vec<&DeviceInfo> = hidapi.device_list().into_iter()
        .filter(|d| d.vendor_id() == ELGATO && alldecks.contains(&d.product_id()))
        .collect(); 

    for i in &devinfo {
        println!("Info: {:?} {:?}", i, i.serial_number())
    }

    if devinfo.is_empty() {
        return Err(DeckError::NoDevice)
    }

    let deviceinfo = devinfo[0];

    match StreamDeck::connect_with_hid(&hidapi, deviceinfo.vendor_id(), deviceinfo.product_id(), deviceinfo.serial_number().map(|s| String::from(s))) {
        Ok(sd) => return Ok(ButtonDevice::Streamdeck(StreamDeckDevice::new(sd))),
        Err(e) => {
            error!("Error connecting to streamdeck: {:?}", e);
            Err(DeckError::NoDevice)
        }
    }




}


pub fn discover_streamdeck(maybe_hidapi: &mut Option<HidApi>) -> Result<ButtonDevice> {

    info!("Discover Streamdeck");
    let hidapi = maybe_hidapi.as_mut().ok_or(DeckError::NoHidApi)?;


    if let Err(e) = hidapi.refresh_devices() {
        error!("{:?}",e);
    }

    let alldecks: Vec<u16> = vec![
        pids::ORIGINAL,
        pids::ORIGINAL_V2,
        pids::MINI,
        pids::XL,
        pids::MK2,
    ];

    let devinfo: Vec<&DeviceInfo> = hidapi.device_list().into_iter()
        .filter(|d| d.vendor_id() == ELGATO && alldecks.contains(&d.product_id()))
        .collect(); 

    for i in &devinfo {
        debug!("Info: {:?} {:?}", i, i.serial_number())
    }



    if devinfo.is_empty() {
        return Err(DeckError::NoDevice)
    }

    let deviceinfo = devinfo[0];


    match StreamDeck::connect_with_hid(&hidapi, deviceinfo.vendor_id(), deviceinfo.product_id(), deviceinfo.serial_number().map(|s| String::from(s))) {
        Ok(sd) => {
            Ok(ButtonDevice::Streamdeck(StreamDeckDevice::new(sd)))
        },
        Err(e) => {
            error!("Error connecting to streamdeck: {:?}", e);
            Err(DeckError::NoDevice)
        }
    }


}