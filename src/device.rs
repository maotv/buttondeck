

use std::{time::Duration, str::FromStr};

use hidapi::{DeviceInfo, HidApi};
use log::{error, info, debug};
use streamdeck::{Colour, ImageOptions};
use ::streamdeck::{pids, StreamDeck};

use crate::DeckEvent;

use super::{DeckError, Button, ButtonColor};


type Result<T> = std::result::Result<T,DeckError>;

const ELGATO: u16 = 0x0fd9;




pub trait ButtonDevice {
    fn wait_for_events(&mut self, timeout: usize) -> Result<Vec<DeckEvent>>;
    fn decorate_button(&mut self, button: &Button) -> Result<()>;
}




pub struct StreamDeckDevice {
    sd: StreamDeck,
    btn_state: [u8;256],
    model: String

}

impl StreamDeckDevice {

    fn new(mut sd: StreamDeck) -> Self {

        let model = sd.product().unwrap_or_else(|e| String::from("unknown")).replace(" ","_").to_lowercase();

        StreamDeckDevice {  
            sd,
            btn_state: [0;256],
            model
        }
    }

    pub fn model(&self) -> String {
        self.model.clone()
//        self.sd.product().unwrap_or_else(|e| String::from("unknown"))
    }
}


fn to_colour(c: &ButtonColor) -> Colour {
    Colour {
        r: ((c.rgb&0xff0000) >> 16) as u8,
        g: ((c.rgb&0x00ff00) >> 8) as u8,
        b: (c.rgb&0x0000ff) as u8,
    }
}

impl ButtonDevice for StreamDeckDevice {
    fn wait_for_events(&mut self, timeout: usize) -> Result<Vec<super::DeckEvent>> {
        read_button_events(self)
    }

    fn decorate_button(&mut self, button: &Button) -> Result<()> {

        debug!("Decorate Button: {:?}", button.index_on_device());
        if let Some(index) = button.index_on_device() {

            let setup = button.current_state();
            
            if let Some(c) = button.effective_color() {
                self.sd.set_button_rgb(index as u8, &to_colour(c))?;
            }
            if let Some(c) = button.effective_image() {
                self.sd.set_button_file(index as u8, &c.path.to_string_lossy(),&ImageOptions::default())?;
            }
        }


        
        
        Ok(())
    }
}



fn read_button_events(sd: &mut StreamDeckDevice) -> Result<Vec<DeckEvent>> {

    let btns = sd.sd.read_buttons(Some(Duration::from_millis(100)));
    let mut events = vec!();

    match btns {
        Ok(b) => {
            debug!("Btn: {:?}", b);
            for i in 0..b.len() {
                if sd.btn_state[i] == 0 && b[i] == 1 {
                    events.push(DeckEvent::ButtonDown(i));
                } else  if sd.btn_state[i] == 1 && b[i] == 0 {
                    events.push(DeckEvent::ButtonUp(i));
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



pub fn open_first_streamdeck(hidapi: &mut HidApi) -> Result<StreamDeckDevice> {

    hidapi.refresh_devices()?;

    let alldecks: Vec<u16> = vec![
        pids::ORIGINAL,
        pids::ORIGINAL_V2,
        pids::MINI,
        pids::XL,
        pids::MK2,
    ];

    let optinfo = hidapi.device_list()
        .filter(|d| d.vendor_id() == ELGATO)
        .find(|d| alldecks.contains(&d.product_id())); 


    if let Some(deviceinfo) = optinfo {
        
        match StreamDeck::connect_with_hid(&hidapi, deviceinfo.vendor_id(), deviceinfo.product_id(), deviceinfo.serial_number().map(|s| String::from(s))) {
            Ok(sd) => Ok(StreamDeckDevice::new(sd)),
            Err(e) => Err(e.into())
        }
    } else {
        Err(DeckError::NoDevice)
    }

}





pub fn open_streamdeck(hidapi: &mut HidApi) -> Result<StreamDeckDevice> {

    info!("Open Streamdeck");

    //  

    let mut optinfo: Option<&DeviceInfo> = None;

    loop {

        while optinfo.is_none() {

            if let Err(e) = hidapi.refresh_devices() {
                error!("{:?}",e);
            }

            let dl = hidapi.device_list();
            optinfo = dl.filter(|d| d.vendor_id() == ELGATO)
                .filter(|d| d.product_id() == pids::MINI)
                .nth(0); 

            if optinfo.is_none() {
                std::thread::sleep(Duration::from_millis(3000))
            }
        }

        if let Some(deviceinfo) = optinfo {
            match StreamDeck::connect_with_hid(&hidapi, deviceinfo.vendor_id(), deviceinfo.product_id(), deviceinfo.serial_number().map(|s| String::from(s))) {
                Ok(sd) => return Ok(StreamDeckDevice::new(sd)),
                Err(e) => {
                    error!("Error connecting to streamdeck: {:?}", e);
                }
            }
        } 

        std::thread::sleep(Duration::from_millis(3000))

    }

}