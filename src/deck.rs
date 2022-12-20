use std::rc::Rc;
use std::time::Duration;
use std::thread;
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::cell::RefCell;
use std::str::FromStr;

use log::{debug, warn, error};

// use crate::ButtonDeviceTrait;
use crate::DeckError;
use crate::device::{PhysicalKey, ButtonDevice};
use crate::device::DeviceEventType;
use crate::device::DeviceEvent;
use std::sync::mpsc::{self, Receiver, Sender};


type Result<T> = std::result::Result<T,DeckError>;


#[derive(Clone,Debug)]
pub struct NamedRef {
    pub id: usize,
    pub name: String
}

impl Default for NamedRef {
    fn default() -> Self {
        Self { id: 0, name: String::from("default") }
    }
}

pub enum DeckEvent {
    ButtonUp(usize),
    ButtonDown(usize),
}



type NoArgFunc = fn() -> ();
type DeckArgsFunc = fn(deck: &mut ButtonDeck, btn: &BtnRef) -> ();

#[derive(Clone)]
pub enum ButtonFn {
    NoArg(String,NoArgFunc),
    DeckArgs(String,DeckArgsFunc)
}

impl ButtonFn {
    fn call_fn(&self, deck: &mut ButtonDeck, btn: &BtnRef) {
        match self {
            ButtonFn::NoArg(_, f) => {
                f()
            },
            ButtonFn::DeckArgs(_, f) => {
                f(deck,btn)
            },
        }
    }
}



// the buttons on the device
#[derive(Default)]
pub struct ButtonSetup {

    pub (crate) reference: NamedRef,
//    pub (crate) name: String,
    pub (crate) mapping: Vec<ButtonMapping>
}

impl Clone for ButtonSetup {
    fn clone(&self) -> Self {
        ButtonSetup {
            reference: self.reference.clone(),
            mapping: self.mapping.iter().map(|b| b.clone()).collect()
        }
    }
}

#[derive(Clone)]
pub struct ButtonMapping {
    pub key:    PhysicalKey,
    pub button: BtnRef
}


pub struct Button
{
    // a private, unique id
    pub (crate) reference:  BtnRef,

    pub (crate) name:  String,
    pub (crate) label: String,

    pub (crate) physical: Option<PhysicalKey>,

    // pub (crate) color: Option<ButtonColor>,
    // pub (crate) image: Option<ButtonImage>,

    pub (crate) defaults: ButtonState,

    pub (crate) default_state: usize,
    pub (crate) current_state: usize,
    pub (crate) states: Vec<ButtonState>,



    // pub (crate) on_button_down: Option<ButtonFn>,
    // pub (crate) on_button_up: Option<ButtonFn>,

    // pub (crate) switch_button_state: Option<NamedRef>,
    // pub (crate) switch_deck_setup: Option<NamedRef>,


}

impl Button {

    fn dump(&self) {
        println!("  Button {}({}) {{", self.name, self.reference.id);
        self.defaults.dump("Defaults");

        println!("    States {{");
        for s in &self.states {
            s.dump("State");
        }
        println!("    }}");

        println!("  }}");
    }

    pub fn assigned_key(&self) -> Option<PhysicalKey> {
        self.physical.clone()
    }

    pub fn current_state<'a>(&'a self) -> &'a ButtonState {
        self.states.get(self.current_state).unwrap_or_else(|| self.states.get(self.default_state).expect("must not go wrong") )
    }

    // pub fn switch_state_by_name(&mut self, next_state: &NamedRef) -> bool 
    // {

    //     let index = match self.states.iter().enumerate().find(|(i,s)| s.name == next_state) {
    //         Some((i,s)) => i,
    //         None => {
    //             warn!("cannot find state '{}'", next_state);
    //             self.current_state
    //         }
    //     };
    
    //     let last = self.current_state;
    //     self.current_state = index;

    //     index != last

    // }

    pub fn get_state_ref(&self, name: &str) -> Option<NamedRef> {
        self.states.iter().find(|s| s.reference.name == name)
            .map(|n| n.reference.clone())
    }

    pub fn switch_state(&mut self, next_state: &NamedRef) -> bool 
    {

        let next = match self.states.get(next_state.id) {
            Some(s) => next_state.id,
            None => self.current_state
        };
    
        let last = self.current_state;
        self.current_state = next;

        next != last

    }


    pub fn switch_state_action(&mut self) -> bool {
        
        let b = match self.effective_switch_button_state() {
            Some(s) => Some(s.clone()),
            None => None
        };

        if let Some(sn) = b {
            self.switch_state(&sn)
        } else {
            false
        }
    }



    pub fn effective_image<'a>(&'a self) -> Option<&'a ButtonImage> {
        match &self.current_state().image {
            Some(c) => Some(c),
            None => match &self.defaults.image {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_color<'a>(&'a self) -> Option<&'a ButtonColor> {
        match &self.current_state().color {
            Some(c) => Some(c),
            None => match &self.defaults.color {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_button_down<'a>(&'a self) -> Option<ButtonFn> {
        match &self.current_state().on_button_down {
            Some(c) => Some(c.clone()),
            None => match &self.defaults.on_button_down {
                Some(c) => Some(c.clone()),
                None => None
            }
        }
    }

    pub fn effective_button_up<'a>(&'a self) -> Option<ButtonFn> {
        match &self.current_state().on_button_up {
            Some(c) => Some(c.clone()),
            None => match &self.defaults.on_button_up {
                Some(c) => Some(c.clone()),
                None => None
            }
        }
    }

    pub fn effective_switch_button_state<'a>(&'a self) -> Option<&'a NamedRef> {
        match &self.current_state().switch_button_state {
            Some(c) => Some(c),
            None => match &self.defaults.switch_button_state {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_switch_deck_setup<'a>(&'a self) -> Option<&'a NamedRef> {
        match &self.current_state().switch_deck_setup {
            Some(c) => Some(c),
            None => match &self.defaults.switch_deck_setup {
                Some(c) => Some(c),
                None => None
            }
        }
    }


}

#[derive(Clone,Debug)]
pub struct ButtonImage {
    pub path: PathBuf
}

impl ButtonImage {
    pub fn from_option_string(folder: &Path, s: &Option<String>) -> Option<Self> {
        if let Some(c) = s {
            Some(ButtonImage {
                path: folder.join(c)
            })
        } else {
            None
        }
    }
}



#[derive(Clone, Debug)]
pub struct ButtonColor {
    pub rgb: u32
}

impl ButtonColor {
    pub fn from_option_string(s: &Option<String>) -> Option<Self> {
        if let Some(c) = s {
            ButtonColor::from_str(c).ok()
        } else {
            None
        }
    }
}

impl FromStr for ButtonColor {
    type Err = DeckError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {

        let hex = s.trim_start_matches("#").trim_start_matches("0x");
        let num = u32::from_str_radix(hex, 16); 

        Ok(ButtonColor {
            rgb: num.unwrap_or(0)
        })

    }
}

#[derive(Default)]
pub struct ButtonState
{

    pub (crate) reference: NamedRef,
    // pub (crate) name: String,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) on_button_down: Option<ButtonFn>,
    pub (crate) on_button_up: Option<ButtonFn>,

    pub (crate) switch_button_state: Option<NamedRef>,
    pub (crate) switch_deck_setup: Option<NamedRef>,

}

impl ButtonState {

    fn dump(&self, title: &str) {
        println!("    {} {}({}) {{", title, self.reference.name, self.reference.id);
        
        // self.defaults.dump();
        println!("    }}");
    }

}


#[derive(Clone,Debug)]
pub struct BtnRef {
    pub (crate) id: usize,
    pub (crate) state: Option<NamedRef>
}

impl BtnRef {
    pub (crate) fn clone_with_state(&self, state: Option<NamedRef>) -> Self {
        BtnRef { 
            id: self.id,
            state
        }
    }
}

impl AsRef<BtnRef> for BtnRef {
    fn as_ref(&self) -> &BtnRef {
        self
    }
}


pub struct ButtonDeck
{

    // pub (crate) device: Rc<RefCell<Box<dyn ButtonDevice>>>,
    pub (crate) device: Option<ButtonDevice>,

    pub (crate) device_sender: Sender<DeviceEvent>,

    pub (crate) device_receiver: Option<Receiver<DeviceEvent>>,


    // folder with config & icons
    pub (crate) folder: PathBuf,

    // The memory arena where all defined buttons live
    pub (crate) button_arena: Vec<Button>,

    // mapping from key-index to Phys & BtnRef
    pub (crate) current_key_map: Vec<Option<ButtonMapping>>,

    // 
    pub (crate) wiring: Vec<Option<PhysicalKey>>,


    // buttons by name
    pub (crate) button_map: HashMap<String,BtnRef>,

    // the current, active setup
    pub (crate) current_setup: usize,

    // all setups by name
    pub (crate) setup_arena: Vec<ButtonSetup>,

    // receiver: mpsc::Receiver<DeviceEvent>

}




pub struct ButtonDeckSender {
    pub sender: Sender<DeviceEvent>
}

impl ButtonDeck 
{

    pub fn initialize(&mut self) -> Result<()> {


        let (tx_to_buttondeck,rx_from_device) = mpsc::channel();

        let optdev = self.device.take();
        debug!("Device is: {:?}", optdev.is_some());


        if let Some(device) = optdev {

            let tx_to_device = match device {
                ButtonDevice::Streamdeck(mut sd) => {
                    sd.start(tx_to_buttondeck)
                },
                ButtonDevice::Midi(mut md) => {
                    md.start(tx_to_buttondeck)
                },
            }?;

            self.device_receiver = Some(rx_from_device);
            self.device_sender   = tx_to_device.clone();



            let nr = self.setup_arena[0].reference.clone();
            self.switch_to(&nr);

            Ok( () )

        } else {
            Err(DeckError::NoDevice)
        }

    }

    pub fn spawn(&mut self) {

    }


    pub fn run(&mut self) {

        let optrx = self.device_receiver.take();
        if optrx.is_none() { return; }

        let rx = optrx.expect("checked above");


        loop {
            if let Ok(event) = rx.recv() {
                match event {
                    DeviceEvent::ButtonDown(index, velocity) => {
                        self.on_button_down(index);
                    }

                    DeviceEvent::ButtonUp(index) => {
                        self.on_button_up(index);
                    }

                    _ => {

                    }
                }
    
            }
        }


    }



    pub fn switch_to_name(&mut self, setup_name: &str) {
        // let su =  self.setup_arena.iter().find(|s| s.name == setup_name);
 
     }
 
     pub fn switch_to_default(&mut self) {

        let sref = self.setup_arena.get(0).cloned();

        if let Some(s) = sref {
            self.switch_to(&s.reference);
        }
//        self.switch_to(&self.setup_arena.get(0).unwrap())
     }
 
  
    pub fn switch_to(&mut self, setup: &NamedRef) {

        debug!("switch_to {:?}", setup);

        // cleanup connections to physical buttons
        for b in &mut self.button_arena {
            b.physical = None;
        }

        if let Some(s) = self.setup_arena.get(setup.id) {
            self.current_setup = setup.id;
        } else {
            warn!("cannot find setup '{}'", setup.name)
        }

        self.init_setup();
    }

    pub fn init_setup(&mut self) {

        debug!("init_setup {}", self.current_setup);

        // FIXME do this without cloning buttonsetup
        if let Some(bs) = self.setup_arena.get(self.current_setup).cloned() {
            for b in &bs.mapping {
                self.init_button(b);
            }
        }

    }

    pub fn init_button(&mut self, mapping: &ButtonMapping) -> Result<()> {

        debug!("init_button {:?} {}", mapping.key, mapping.button.id);
        self.current_key_map[mapping.key.id] = Some(mapping.clone());
        {
            let bb = self.button_mut(&mapping.button)?;
            bb.physical = Some(mapping.key.clone());

            // let km = self.wiring.get_mut(mapping.key.id);

            if let Some(bs) = &mapping.button.state {
                debug!("=> Switch state to {:?}", bs);
                bb.switch_state(bs);
            }
        }

        self.decorate_button(&mapping.button)

    }


    pub fn decorate_button(&self, btn: &BtnRef) -> Result<()> {

        debug!("decorate_button {:?}", &btn);

        let button = self.button(btn)?;
        let color = button.effective_color();
        let image = button.effective_image();

        let key = button.assigned_key();
        if let Some(pk) = key {
            debug!("key is {:?}", &pk);
            if let Some(c) = color {
                self.device_sender.send(DeviceEvent::SetColor(pk.id, c.clone()));
            }
            if let Some(c) = image {
                debug!("image is {:?}", &c);
                self.device_sender.send(DeviceEvent::SetImage(pk.id, c.clone()));
            }
        }

        Ok(())
    }


    fn button<R: AsRef<BtnRef>>(&self, r: R) -> Result<&Button> {
        self.button_arena.get(r.as_ref().id).ok_or(DeckError::InvalidRef)
    }

    fn button_mut<R: AsRef<BtnRef>>(&mut self, r: R) -> Result<&mut Button> {
        self.button_arena.get_mut(r.as_ref().id).ok_or(DeckError::InvalidRef)
    }




    fn on_button_down(&mut self, index: usize) -> Result<()> {

        debug!("on_button_down #{}", index);
        
        let btn = match self.current_key_map.get(index) {
            Some(om) => match om {
                Some(m) => Some(m.button.clone()),
                None => None
            }
            None => None
        };
        
        
        if let Some(br) = btn {
            
            if let Some(bdf) = self.button(&br)?.effective_button_down() {
                bdf.call_fn(self,&br);
            }

            
            let switched = self.button_mut(&br)?.switch_state_action();
            if switched {
                self.decorate_button(&br)?;
            }


            // let setup =  {
            //     Some(s) => Some(String::from(s)),
            //     None => None
            // };

            if let Some(s) = self.button_mut(&br)?.effective_switch_deck_setup().cloned() {
                self.switch_to(&s);
            }

        }

       
        Ok(())
    }

    fn on_button_up(&mut self, index: usize) -> Result<()> {

        debug!("on_button_up #{}", index);

        let btn = match self.current_key_map.get(index) {
            Some(om) => match om {
                Some(m) => Some(m.button.clone()),
                None => None
            }
            None => None
        };

        if let Some(br) = btn {
            if let Some(bdf) = self.button(&br)?.effective_button_up() {
                bdf.call_fn(self,&br);
            }
        }

        Ok(())
    }



}




