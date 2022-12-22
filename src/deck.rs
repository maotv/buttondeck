
use log::error;
use log::{debug, warn};

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use std::rc::Rc;
use std::str::FromStr;


use crate::{ButtonRef, ButtonColor, StateRef2};
use crate::Button;
use crate::DeckError;
use crate::device::ButtonDevice;
use crate::device::PhysicalKey;
use crate::device::DeviceEvent;
use std::sync::mpsc::{self, Receiver, Sender};


type Result<T> = std::result::Result<T,DeckError>;


pub enum DeckEvent {
    Void,
    Device(DeviceEvent),
    FnCall(String, FnArg)
}



#[derive(Clone,Debug)]
pub struct FnRef {
    pub id:   usize,
    pub name: String,
}


#[derive(Clone,Debug)]
pub struct SetupRef {
    pub id: usize,
    pub name: String
}

impl Default for SetupRef {
    fn default() -> Self {
        Self { id: 0, name: String::from("default") }
    }
}



// #[derive(Clone,Debug)]
// pub struct NamedRef {
//     pub id: usize,
//     pub name: String
// }

// impl Default for NamedRef {
//     fn default() -> Self {
//         Self { id: 0, name: String::from("default") }
//     }
// }




// pub enum DeckEvent {
//     ButtonUp(usize),
//     ButtonDown(usize),
// }

// pub struct DeckEvent<'a> {
//     button: &'a ButtonRef
// }



type NoArgFunc = dyn Fn() -> () + 'static;
type DeckArgsFunc = dyn FnMut(&mut ButtonDeck, FnArg) -> Result<()> + 'static;

// #[derive(Clone)]
pub struct ButtonFn {
    pub func: Box<DeckArgsFunc>
}
// impl From<NoArgFunc> for ButtonFn {
//     fn from(x: NoArgFunc) -> Self {
//         todo!()
//     }
// }

impl ButtonFn {
    fn call_fn(&mut self, deck: &mut ButtonDeck, arg: FnArg) {
        if let Err(e) = (self.func)(deck,arg) {
            error!("ButtonFn returned error: {:?}", e);
        }
    }
}



/// the buttons on the device
#[derive(Default)]
pub struct ButtonSetup {

    pub (crate) reference: SetupRef,
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
    pub button: ButtonRef,
    pub state:  Option<StateRef2>
}


pub struct ButtonDeckSender {
    pub sender: Sender<DeckEvent>
}

impl ButtonDeckSender {
    pub fn send(&self, event: DeckEvent) {
        self.sender.send(event);
    }
}


pub enum FnArg {
    None,
    Bool(bool),
    Int(isize),
    Float(f32),
    Button(ButtonRef),
}


impl FnArg {
    pub fn as_bool(&self) -> bool {
        match self {
            FnArg::Bool(b) => *b,
            _ => false 
        }
    }
}



impl Display for FnArg {
    fn fmt(&self, fm: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnArg::Bool(b) => write!(fm, "FnArg::Bool({})", b),
            FnArg::Int(i) => write!(fm, "FnArg::Int({})", i),
            FnArg::Float(f) => write!(fm, "FnArg::Float({})", f),
            FnArg::Button(b) => write!(fm, "FnArg::Button({:?})", b),
            FnArg::None => write!(fm, "FnArg::None")
        }
    }
}


pub struct ButtonDeck
{

    pub (crate) deckid: usize,

    // pub (crate) device: Rc<RefCell<Box<dyn ButtonDevice>>>,
    pub (crate) device: Option<ButtonDevice>,

    pub (crate) device_sender:   Sender<DeviceEvent>,

    pub (crate) device_receiver: Option<Receiver<DeckEvent>>,

    pub (crate) self_sender:     Sender<DeckEvent>,


    pub (crate) functions: Vec<(String,Rc<RefCell<ButtonFn>>)>,


    // folder with config & icons
    pub (crate) folder: PathBuf,

    // The memory arena where all defined buttons live
    pub (crate) button_arena: Vec<Button>,

    // mapping from key-index to Phys & ButtonRef
    pub (crate) current_key_map: Vec<Option<ButtonMapping>>,

    // 
    pub (crate) wiring: Vec<Option<PhysicalKey>>,


    // buttons by name
    pub (crate) button_map: HashMap<String,ButtonRef>,

    // the current, active setup
    pub (crate) current_setup: usize,

    // all setups by name
    pub (crate) setup_arena: Vec<ButtonSetup>,

    // receiver: mpsc::Receiver<DeviceEvent>

}


impl ButtonDeck
{

    pub fn initialize(&mut self) -> Result<()> {


        let (tx_to_buttondeck,rx_from_devices) = mpsc::channel();

        let optdev = self.device.take();
        debug!("Device is: {:?}", optdev.is_some());


        if let Some(device) = optdev {

            let tx_to_device = match device {
                ButtonDevice::Streamdeck(mut sd) => {
                    sd.start(tx_to_buttondeck.clone())
                },
                ButtonDevice::Midi(mut md) => {
                    md.start(tx_to_buttondeck.clone())
                },
            }?;

            self.device_receiver = Some(rx_from_devices);
            self.device_sender   = tx_to_device.clone();
            self.self_sender = tx_to_buttondeck.clone();



            let nr = self.setup_arena[0].reference.clone();
            self.switch_to_ref(&nr);

            Ok( () )

        } else {
            Err(DeckError::NoDevice)
        }

    }

    pub fn get_sender(&self) -> ButtonDeckSender {
        ButtonDeckSender {
            sender: self.self_sender.clone()
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
                    DeckEvent::Void => {
                        warn!("Got void event");
                    },
                    DeckEvent::FnCall(name, arg) => {
                        self.call_fn_by_name(&name, arg)
                    },
                    DeckEvent::Device(e) => {
                        self.handle_device_event(e)
                    },
                }
            }
        }


    }

    fn handle_device_event(&mut self, event: DeviceEvent) {
        match event {
            DeviceEvent::ButtonDown(index, velocity) => {
                self.on_button_down(index);
            }

            DeviceEvent::ButtonUp(index) => {
                self.on_button_up(index);
            }

            _ => {
                warn!("Unhandled DeviceEvent: {:?}", event)
            }
        }

    }




    pub fn switch_to(&mut self, setup_name: &str) {
        let su =  self.setup_arena.iter().map(|s| &s.reference).find(|s| s.name == setup_name).cloned();
        if let Some(r) = su {
            self.switch_to_ref(&r)
        }        
     }
 
     pub fn switch_to_default(&mut self) {

        let sref = self.setup_arena.get(0).cloned();

        if let Some(s) = sref {
            self.switch_to_ref(&s.reference);
        }

    }
 
  
    pub fn switch_to_ref(&mut self, setup: &SetupRef) {

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

    fn init_button(&mut self, mapping: &ButtonMapping) -> Result<()> {

        debug!("init_button {:?} {:?}", mapping.key, mapping.button);

        self.current_key_map[mapping.key.id] = Some(mapping.clone());
        {
            let bb = self.button_mut(&mapping.button)?;
            bb.physical = Some(mapping.key.clone());

            // let km = self.wiring.get_mut(mapping.key.id);

            if let Some(bs) = &mapping.state {
                debug!("=> Switch state to {:?}", bs);
                bb.switch_state2(bs);
            }
        }

        self.decorate_button(&mapping.button)

    }


    fn decorate_button(&self, btn: &ButtonRef) -> Result<()> {

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

    pub fn toggle_button_state(&mut self, rb: &ButtonRef) -> Result<()> {
        let b = self.button_mut(rb)?;
        if b.toggle_state() {
            self.decorate_button(rb)?;
        }
        Ok(())
    }

    pub fn set_button_state<'a, R, S>(&mut self, button: R, state: S) 
        where   R: AsRef<ButtonRef>,
                S: AsRef<StateRef2>
    {
        // if let Some(rb) = self.button_ref(button_name) {
        //     if let Ok(button) = self.button_mut(rb) {
        //         // FIXME button.switch_state(next_state)
        //     }
        // }
    }

    pub fn set_button_color(&mut self, button: ButtonRef, state: StateRef2, color: ButtonColor) {
    }

    pub fn set_button_color2(&mut self, button: &ButtonRef, state: &str, color: ButtonColor) {
    }




    fn call_fn_by_name(&mut self, name: &str, arg: FnArg) {

        debug!("call_fn_by_name {}", name);

        // for f in  &self.functions {
        //     debug!("    available: {}", f.0);
        // }

        let opt_func = self.functions.iter().find(|(n,b)| n == name ).map(|(n,f)| f).cloned();
        // debug!("    opt_func is some: {}", opt_func.is_some());
        
        if let Some(f) = opt_func {
            debug!("    call_fn");
            f.borrow_mut().call_fn(self,arg);
        } else {
            warn!("Missing Function: {}", name);
        }

    }



    fn call_fn(&mut self, fr: &FnRef, br: &ButtonRef) {

        let opt_func = self.functions.get(fr.id).cloned(); // .unwrap().clone();
        let arg = FnArg::Button(br.clone());

        if let Some(f) = opt_func {
            f.1.borrow_mut().call_fn(self,arg);
        }
        

    }

    pub fn button_ref(&self, button: &str) -> Option<ButtonRef> {
        self.button_map.get(button).cloned()
//        self.button_arena.iter().find(|b| b.reference.name == button)
    }

    pub fn button_id(&self, button: &ButtonRef) -> Result<usize> {

        match button {
            ButtonRef::Id(owner, index) => {
                if *owner != self.deckid { return Err(DeckError::InvalidRef) }
                Ok(*index)
            },
            ButtonRef::Name(n) => {
                match self.button_map.get(n).cloned().ok_or(DeckError::InvalidRef)? {
                    ButtonRef::Id(_, id) => Ok(id),
                    ButtonRef::Name(_) => Err(DeckError::InvalidRef),
                }
            },
        }
    }


    fn button<R: AsRef<ButtonRef>>(&self, r: R) -> Result<&Button> {
        self.button_arena
            .get(self.button_id(r.as_ref())?)
            .ok_or(DeckError::InvalidRef)
    }

    // fn buttonx(&self, r: &ButtonRef) -> Result<&Button> {

    //     let y = self.button_arena
    //         .get(self.button_id(r)?)
    //         .ok_or(DeckError::InvalidRef);

    //     y
    // }

    fn button_mut<R: AsRef<ButtonRef>>(&mut self, r: R) -> Result<&mut Button> {

        let id = self.button_id(r.as_ref())?;

        self.button_arena
            .get_mut(id)
            .ok_or(DeckError::InvalidRef)

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

            let opt_fr = self.button(&br)?.effective_button_down().cloned();
            
            if let Some(fr) = opt_fr {
                self.call_fn(&fr, &br);
            }

            
            let switched = self.button_mut(&br)?.switch_state_action();
            if switched {
                self.decorate_button(&br)?;
            }

            if let Some(s) = self.button_mut(&br)?.effective_switch_deck_setup().cloned() {
                self.switch_to_ref(&s);
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

            let opt_fr = self.button(&br)?.effective_button_up().cloned();
            
            if let Some(fr) = opt_fr {
                self.call_fn(&fr, &br);
            }
        }

            // if let Some(fr) = self.button(&br)?.effective_button_up() {
            //     if let Some(func) = self.functions.get(fr.id) {
            //         func.call_fn(self,&br);
            //     }
            // }
        // }
        // if let Some(br) = btn {
        //     if let Some(bdf) = self.button(&br)?.effective_button_up() {
        //         bdf.call_fn(self,&br);
        //     }
        // }

        Ok(())
    }



}




