
use hidapi::HidApi;
use log::error;
use log::{debug, warn};

use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use std::rc::Rc;
use std::str::FromStr;
use std::thread;
use std::time::Duration;


use crate::button::{ButtonValue, ButtonImage};
use crate::{ButtonId, ButtonColor, StateRef2, ButtonDeviceTrait, ButtonDeckBuilder};
use crate::Button;
use crate::DeckError;
use crate::device::{ButtonDevice, discover_streamdeck};
use crate::device::PhysicalKey;
use crate::device::DeviceEvent;
use std::sync::mpsc::{self, Receiver, Sender};


type Result<T> = std::result::Result<T,DeckError>;


#[derive(Debug)]
pub enum DeckEvent {
    Void,
    Disconnected,
    Device(DeviceEvent),
    FnCall(String, FnArg),
    SetState(String,String),
    SetImage(String,Option<ButtonImage>),
    SetValue(String,ButtonValue)
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
//     button: &'a ButtonId
// }



type NoArgFunc = dyn Fn() -> () + 'static;
type DeckArgsFunc<D> = dyn FnMut(&mut ButtonDeck<D>, FnArg) -> Result<()> + Send + Sync;

// #[derive(Clone)]
pub struct ButtonFn<D> 
    where D: Send + Sync + 'static
{

    pub func: Box<DeckArgsFunc<D>>
}
// impl From<NoArgFunc> for ButtonFn {
//     fn from(x: NoArgFunc) -> Self {
//         todo!()
//     }
// }

impl <D> ButtonFn<D> 
    where D: Send + Sync + 'static
{
    fn call_fn(&mut self, deck: &mut ButtonDeck<D>, arg: FnArg) {
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
    pub button: ButtonId,
    pub state:  Option<StateRef2>
}


pub struct ButtonDeckSender {
    pub sender: Sender<DeckEvent>
}

impl ButtonDeckSender {
    
    pub fn send(&self, event: DeckEvent) {
        self.sender.send(event);
    }

    pub fn set_image_from_file(&self, button: &str, file: &str) -> Result<()> {
        let image = ButtonImage::from_path(file);
        self.send(DeckEvent::SetImage(String::from(button), image));
        Ok(())
    }

    pub fn set_image(&self, button: &str, image: Option<ButtonImage>) -> Result<()> {
        self.send(DeckEvent::SetImage(String::from(button), image));
        Ok(())
    }

    pub fn set_value(&self, button: &str, value: ButtonValue) -> Result<()> {
        self.send(DeckEvent::SetValue(String::from(button), value));
        Ok(())
    }

}


#[derive(Debug)]
pub enum FnArg {
    None,
    Bool(bool),
    Int(isize),
    Float(f32),
    Button(ButtonId, ButtonValue),
}


impl FnArg {
    pub fn as_bool(&self) -> bool {
        match self {
            FnArg::Bool(b) => *b,
            _ => false 
        }
    }
    pub fn value_to_string(&self) -> String {
        match self {
            FnArg::Button(b, v) => v.to_string(),
            _ => String::new() 
        }
    }
}



impl Display for FnArg {
    fn fmt(&self, fm: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnArg::Bool(b) => write!(fm, "FnArg::Bool({})", b),
            FnArg::Int(i) => write!(fm, "FnArg::Int({})", i),
            FnArg::Float(f) => write!(fm, "FnArg::Float({})", f),
            FnArg::Button(b, v) => write!(fm, "FnArg::Button({:?}, {:?})", b, v),
            FnArg::None => write!(fm, "FnArg::None")
        }
    }
}


// device dependent fields
pub struct DeckDeviceSetup {

    // pub (crate) device: Rc<RefCell<Box<dyn ButtonDevice>>>,
    pub (crate) device: Option<ButtonDevice>,

    // The memory arena where all defined buttons live
    pub (crate) button_arena: Vec<Button>,

    // mapping from key-index to Phys & ButtonId
    pub (crate) current_key_map: Vec<Option<ButtonMapping>>,

    // 
    pub (crate) wiring: Vec<Option<PhysicalKey>>,

    // all setups by name
    pub (crate) setup_arena: Vec<ButtonSetup>,

    // the current, active setup
    pub (crate) current_setup: usize,



}

impl Default for DeckDeviceSetup {
    fn default() -> Self {
        Self { 
            device: None,
            button_arena: Default::default(), 
            current_key_map: Default::default(), 
            wiring: Default::default(), 
            setup_arena: Default::default() ,
            current_setup: 0
        }
    }
}


pub struct ButtonDeck<D>
    where D: Send + Sync + 'static
{

    pub (crate) deckid: usize,

    pub (crate) builder: ButtonDeckBuilder<D>,

    pub (crate) hidapi: Option<HidApi>,


    // folder with config & icons
    pub (crate) folder: PathBuf,

    // sender to button device
    pub (crate) device_event_sender:   Sender<DeviceEvent>,

    // ??? 
    pub (crate) deck_event_receiver: Option<Receiver<DeckEvent>>,

    // sender to deck (self)
    pub (crate) deck_event_sender:     Sender<DeckEvent>,


    pub (crate) functions: Vec<(String,Rc<RefCell<ButtonFn<D>>>)>,
    // pub (crate) func_refs: Vec<FnRef>,


    pub (crate) ddsetup: DeckDeviceSetup,

    pub data: Option<D>,

    pub other: Option<Box<dyn Any>>

    // receiver: mpsc::Receiver<DeviceEvent>

}


impl <D> ButtonDeck<D>
    where D: Send + Sync + 'static
{

    pub fn with_other<X>(&mut self, o: X) 
        where X: Sized + Send + Sync + 'static
    {
        self.other = Some(Box::new(o));
    }


    // pub fn new(b: ButtonDeckBuilder<D>) -> Self {

    //     let (dvtx,dvrx) = std::sync::mpsc::channel::<DeviceEvent>(); 
    //     let (bdtx,bdrx) = std::sync::mpsc::channel::<DeckEvent>();
    
    //     ButtonDeck {

    //         deckid: 0,

    //         builder: b,

    //         hidapi: None, // will be filled later

    //         // device: xdev, // Rc::new(RefCell::new(Box::new(device))),
    //         folder: PathBuf::default(),

    //         ddsetup: Default::default(),

    //         functions: Vec::new(),

    //         // func_refs: Vec::new(),

    //         // dummy!
    //         device_event_sender: dvtx,
    //         // dummy!
    //         deck_event_sender: bdtx,
    //         // dummy!
    //         deck_event_receiver: Some(bdrx),

    //         data: None,
            
    //         other: None,

    //     }
    // }


    // pub fn initialize(&mut self) -> Result<()> {


    //     let (tx_deck_events,rx_deck_events) = mpsc::channel();

    //     let optdev = self.device.take();
    //     debug!("Device is: {:?}", optdev.is_some());


    //     if let Some(device) = optdev {

    //         let tx_to_device = match device {
    //             ButtonDevice::Streamdeck(mut sd) => {
    //                 sd.start(tx_deck_events.clone())
    //             },
    //             ButtonDevice::Midi(mut md) => {
    //                 md.start(tx_deck_events.clone())
    //             },
    //         }?;

    //         self.device_event_sender   = tx_to_device.clone();

    //         self.deck_event_receiver = Some(rx_deck_events);
    //         self.deck_event_sender = tx_deck_events;



    //         let nr = self.ddsetup.setup_arena[0].reference.clone();
    //         self.switch_to_ref(&nr);

    //         Ok( () )

    //     } else {
    //         Err(DeckError::NoDevice)
    //     }

    // }

    pub fn get_sender(&self) -> ButtonDeckSender {
        ButtonDeckSender {
            sender: self.deck_event_sender.clone()
        }
    }

    pub fn run(&mut self) {


        let tx_device_to_deck = self.deck_event_sender.clone();
        let receiver = self.deck_event_receiver.take().expect("it is fatal if we dont have a receiver here");
        // self.deck_event_sender = tx;

        loop {
            
            if let Err(e) = self.run_once(&receiver, tx_device_to_deck.clone()) {
                error!("buttondeck.run error: {:?}", e);
            }

            thread::sleep(Duration::from_millis(3000));
        }

    }

 
    fn run_once(&mut self, rx: &Receiver<DeckEvent>, tx_device_to_deck: Sender<DeckEvent>) -> Result<()> {


        let device = self.run_reconnect();
        let mut dds = self.builder.build_for_device(device)?;


        let opt_device = dds.device.take();
        self.ddsetup = dds;

        if let Some(device) = opt_device {
            match device.start(tx_device_to_deck) {
                Ok(s) => self.device_event_sender = s,
                Err(e) => error!("device start error {:?}", e),
            }
        }
        
        let nr = self.ddsetup.setup_arena[0].reference.clone();
        self.switch_to_ref(&nr);

        loop {

            match rx.recv() {

                Ok(event) => {
                    debug!("Got event: {:?}", event);
                    match self.run_event(event) {
                        Ok(_) => (),
                        Err(DeckError::Disconnected) => {
                            break;
                        }
                        Err(e) => {
                            error!("event handling error: {:?}", e)
                        },
                    }
                }
                Err(e) => {
                    error!("event recv error: {:?}", e)
                },
            }
        }

        Ok(())

    }
   



    fn run_event(&mut self, event: DeckEvent) -> Result<()>{

        warn!("DeckEvent ...");
        
        match event {

            DeckEvent::Void => {
                warn!("Got void event");
            },
            DeckEvent::FnCall(name, arg) => {
                self.call_fn_by_name(&name, arg)
            },
            DeckEvent::SetState(name, state) => {
                warn!("Got set_button_state event {} {}", name, state);
                self.set_button_state(&name, &state);
            },
            DeckEvent::Device(e) => {
                self.handle_device_event(e)
            },
            DeckEvent::Disconnected => {
                debug!("Disconnected...");
                return Err(DeckError::Disconnected);
            }
            DeckEvent::SetImage(name, image) => {
                self.set_button_icon(&name, "default", image)?;
            },
            DeckEvent::SetValue(name, value) => {
                self.set_button_value(&name, "default", value)?;
            },
        }

        Ok(())
    }





    fn run_reconnect(&mut self) -> ButtonDevice {

        loop {
            debug!("Reconnect Loop...");
           
            match discover_streamdeck(&mut self.hidapi) {
                Ok(sd) => { 
                    debug!("found device!!! {}", sd.model());
                    return sd;
                },
                Err(e) => {
                    warn!("e? {:?}",e)
                },
            }

            thread::sleep(Duration::from_millis(3000));
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
        let su =  self.ddsetup.setup_arena.iter().map(|s| &s.reference).find(|s| s.name == setup_name).cloned();
        if let Some(r) = su {
            self.switch_to_ref(&r)
        }        
     }
 
     pub fn switch_to_default(&mut self) {

        let sref = self.ddsetup.setup_arena.get(0).cloned();

        if let Some(s) = sref {
            self.switch_to_ref(&s.reference);
        }

    }
 
  
    pub fn switch_to_ref(&mut self, setup: &SetupRef) {

        debug!("switch_to {:?}", setup);

        // cleanup connections to physical buttons
        for b in &mut self.ddsetup.button_arena {
            b.physical = None;
        }

        if let Some(s) = self.ddsetup.setup_arena.get(setup.id) {
            self.ddsetup.current_setup = setup.id;
        } else {
            warn!("cannot find setup '{}'", setup.name)
        }

        self.init_setup();
    }

    pub fn init_setup(&mut self) {

        debug!("init_setup {}", self.ddsetup.current_setup);

        // FIXME do this without cloning buttonsetup
        if let Some(bs) = self.ddsetup.setup_arena.get(self.ddsetup.current_setup).cloned() {
            for b in &bs.mapping {
                self.init_button(b);
            }
        }

    }

    fn init_button(&mut self, mapping: &ButtonMapping) -> Result<()> {

        debug!("init_button {:?} {:?}", mapping.key, mapping.button);

        self.ddsetup.current_key_map[mapping.key.id] = Some(mapping.clone());
        {
            let bb = self.button_mut(mapping.button)?;
            bb.physical = Some(mapping.key.clone());

            // let km = self.wiring.get_mut(mapping.key.id);

            if let Some(bs) = &mapping.state {
                debug!("=> Switch state to {:?}", bs);
                bb.switch_state2(bs);
            }
        }

        self.decorate_button(mapping.button)

    }



    fn decorate_button(&self, btn: ButtonId) -> Result<()> {

        debug!("decorate_button {:?}", &btn);

        let button = self.button(btn)?;
        let color = button.effective_color();
        let image = button.effective_image();

        let key = button.assigned_key();
        if let Some(pk) = key {
            debug!("key is {:?}", &pk);
            if let Some(c) = color {
                self.device_event_sender.send(DeviceEvent::SetColor(pk.id, c.clone()));
            }
            if let Some(c) = image {
                debug!("image is {:?}", &c);
                self.device_event_sender.send(DeviceEvent::SetImage(pk.id, c.clone()));
            }
        }

        Ok(())
    }

    pub fn toggle_button_state(&mut self, rb: ButtonId) -> Result<()> {
        let b = self.button_mut(rb)?;
        if b.toggle_state() {
            self.decorate_button(rb)?;
        }
        Ok(())
    }

    pub fn set_button_state(&mut self, name: &str, state: &str) -> Result<()> {

        let bid = self.button_id_from_name(name)?;

        if let Ok(b) = self.button_mut(bid) {
            if b.switch_state(state) {
                self.decorate_button(bid);
            }
        }


        Ok(())
    }


    pub fn set_button_state_with_id(&mut self, button: ButtonId, state: &StateRef2) 
    {

//         let bref = button.

        if let Ok(b) = self.button_mut(button) {
            if b.switch_state2(state.as_ref()) {
                self.decorate_button(button);
            }
        }
        

        // if let Some(rb) = self.button_ref(button_name) {
        //     if let Ok(button) = self.button_mut(rb) {
        //         // FIXME button.switch_state(next_state)
        //     }
        // }
    }

    pub fn set_button_icon(&mut self, button: &str, state_name: &str, icon: Option<ButtonImage>) -> Result<()> {

        let bid = self.button_id_from_name(button)?;

        if let Ok(b) = self.button_mut(bid) {
            b.set_state_image(state_name, icon);

        }

        self.decorate_button(bid)?;
        Ok(())

    }

    pub fn set_button_value(&mut self, button: &str, state_name: &str, value: ButtonValue) -> Result<()> {

        let bid = self.button_id_from_name(button)?;

        if let Ok(b) = self.button_mut(bid) {
            b.set_state_value(state_name, value);

        }

        Ok(())

    }


    pub fn set_button_color(&mut self, button: ButtonId, state: StateRef2, color: ButtonColor) {
    }

    pub fn set_button_color2(&mut self, button: ButtonId, state: &str, color: ButtonColor) {
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



    fn call_fn(&mut self, fr: &FnRef, br: ButtonId) {

        let val = self.button(br).and_then(|b| Ok(b.effective_value())).unwrap_or(&ButtonValue::None);


        let opt_func = self.functions.get(fr.id).cloned(); // .unwrap().clone();
        let arg = FnArg::Button(br.clone(), val.clone());

        if let Some(f) = opt_func {
            f.1.borrow_mut().call_fn(self,arg);
        }
        

    }

    pub fn button_id_from_name(&self, bname: &str) -> Result<ButtonId> {
        // self.button_map.get(button).cloned()
        self.ddsetup.button_arena.iter().enumerate()
            .find(|(i,b)| b.name == bname)
            .map(|(i,b)| ButtonId::new(self.deckid, i))
            .ok_or_else(|| DeckError::InvalidRef)
    }

    // pub fn button_id(&self, button: ButtonId) -> Result<usize> {

    //     match button {
    //         ButtonId::Id(owner, index) => {
    //             if *owner != self.deckid { return Err(DeckError::InvalidRef) }
    //             Ok(*index)
    //         },
    //         ButtonId::Name(n) => {
    //             match self.button_id_from_name(n) {
    //                 Some(r) => Ok(r),
    //                 None => Err(DeckError::InvalidRef)
    //             }
    //         },
    //     }
    // }

    // fn id_ref(&self, button: ButtonId) -> Result<ButtonId> {
    //     match button {
    //         ButtonId::Id(..) => Ok(button),
    //         ButtonId::Name(n) => {
    //             match self.button_id_from_name(&n) {
    //                 Some(r) => Ok(ButtonId::Id(self.deckid, r)),
    //                 None => Err(DeckError::InvalidRef)
    //             }
    //         },
    //     }
    // }



    fn button(&self, id: ButtonId) -> Result<&Button> {
        self.ddsetup.button_arena
            .get(id.id())
            .ok_or(DeckError::InvalidRef)
    }

    fn button_mut(&mut self, id: ButtonId) -> Result<&mut Button> {


        self.ddsetup.button_arena
            .get_mut(id.id())
            .ok_or(DeckError::InvalidRef)

    }

    fn on_button_down(&mut self, index: usize) -> Result<()> {

        debug!("on_button_down #{}", index);
        
        let btn = match self.ddsetup.current_key_map.get(index) {
            Some(om) => match om {
                Some(m) => Some(m.button.clone()),
                None => None
            }
            None => None
        };
        
        
        if let Some(br) = btn {

            debug!("on_button_down id={:?}", br);

            let opt_fr = self.button(br)?.effective_button_down().cloned();
            
            if let Some(fr) = opt_fr {
                debug!("a");
                self.call_fn(&fr, br);
            }

            debug!("aa");
            
            let switched = self.button_mut(br)?.switch_state_action();
            debug!("ab");
            if switched {
                debug!("b");
                self.decorate_button(br)?;
            }

            if let Some(s) = self.button_mut(br)?.effective_switch_deck_setup().cloned() {
                debug!("c");

                self.switch_to_ref(&s);
            }

        }

       
        Ok(())

    }

    fn on_button_up(&mut self, index: usize) -> Result<()> {

        debug!("on_button_up #{}", index);

        let btn = match self.ddsetup.current_key_map.get(index) {
            Some(om) => match om {
                Some(m) => Some(m.button.clone()),
                None => None
            }
            None => None
        };


        if let Some(br) = btn {

            let opt_fr = self.button(br)?.effective_button_up().cloned();
            
            if let Some(fr) = opt_fr {
                self.call_fn(&fr, br);
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




