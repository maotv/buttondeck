use std::{str::FromStr, rc::Rc, cell::RefCell, path::{PathBuf, Path}, collections::HashMap};

use log::debug;

use crate::{DeckError, ButtonDevice};



type Result<T> = std::result::Result<T,DeckError>;



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
    pub (crate) name: String,
    pub (crate) buttons: Vec<BtnRef>
}

impl Clone for ButtonSetup {
    fn clone(&self) -> Self {
        ButtonSetup {
            name: self.name.clone(),
            buttons: self.buttons.iter().map(|b| b.clone()).collect()
        }
    }
}


pub struct Button
{
    pub (crate) name: String,
    pub (crate) label: String,

    pub (crate) index: Option<usize>,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) current_state: usize,
    pub (crate) states: Vec<ButtonState>,

    pub (crate) on_button_down: Option<ButtonFn>,
    pub (crate) on_button_up: Option<ButtonFn>,

    pub (crate) switch_button_state: Option<String>,
    pub (crate) switch_deck_setup: Option<String>,

    pub (crate) default_state: ButtonState // will (hopefully never be used) - make this private!!!!

}

impl Button {

    pub fn index_on_device(&self) -> Option<usize> {
        self.index
    }

    pub fn current_state<'a>(&'a self) -> &'a ButtonState {
        self.states.get(self.current_state).unwrap_or_else(|| &self.default_state)
    }

    pub fn switch_state(&mut self, next_state: &str) -> bool 
    {

        let index = match self.states.iter().enumerate().find(|(i,s)| s.name == next_state) {
            Some((i,s)) => i,
            None => self.current_state
        };
    
        let last = self.current_state;
        self.current_state = index;

        index != last

    }

    pub fn switch_state_action(&mut self) -> bool {
        
        let b = match self.effective_switch_button_state() {
            Some(s) => Some(String::from(s)),
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
            None => match &self.image {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_color<'a>(&'a self) -> Option<&'a ButtonColor> {
        match &self.current_state().color {
            Some(c) => Some(c),
            None => match &self.color {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_button_down<'a>(&'a self) -> Option<ButtonFn> {
        match &self.current_state().on_button_down {
            Some(c) => Some(c.clone()),
            None => match &self.on_button_down {
                Some(c) => Some(c.clone()),
                None => None
            }
        }
    }

    pub fn effective_button_up<'a>(&'a self) -> Option<ButtonFn> {
        match &self.current_state().on_button_up {
            Some(c) => Some(c.clone()),
            None => match &self.on_button_up {
                Some(c) => Some(c.clone()),
                None => None
            }
        }
    }

    pub fn effective_switch_button_state<'a>(&'a self) -> Option<&'a str> {
        match &self.current_state().switch_button_state {
            Some(c) => Some(c),
            None => match &self.switch_button_state {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_switch_deck_setup<'a>(&'a self) -> Option<&'a str> {
        match &self.current_state().switch_deck_setup {
            Some(c) => Some(c),
            None => match &self.switch_deck_setup {
                Some(c) => Some(c),
                None => None
            }
        }
    }


}

#[derive(Clone)]
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



#[derive(Clone)]
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

    pub (crate) name: String,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) on_button_down: Option<ButtonFn>,
    pub (crate) on_button_up: Option<ButtonFn>,

    pub (crate) switch_button_state: Option<String>,
    pub (crate) switch_deck_setup: Option<String>,

}

#[derive(Clone)]
pub struct BtnRef {
    pub (crate) id: usize,
    pub (crate) state: Option<String>
}

impl BtnRef {
    pub (crate) fn clone_with_state(&self, state: Option<String>) -> Self {
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

    pub (crate) device: Rc<RefCell<Box<dyn ButtonDevice>>>,

    // folder with config & icons
    pub (crate) folder: PathBuf,

    // The memory arena where all defined buttons live
    pub (crate) arena: Vec<Button>,

    // buttons by name
    pub (crate) button_map: HashMap<String,BtnRef>,

    // the current, active setup
    pub (crate) setup: ButtonSetup,

    // all setups by name
    pub (crate) setup_map: HashMap<String,ButtonSetup>


}

impl ButtonDeck 
{


    pub fn run(&mut self) {
        button_deck_loop(self);
    }


    pub fn switch_to(&mut self, setup: &str) {

        debug!("switch_to {}", setup);

        // cleanup connections to physical buttons
        for b in &mut self.arena {
            b.index = None;
        }

        if let Some(s) = self.setup_map.get(setup) {
            self.setup = s.clone();
        }

        self.init_setup();
    }

    pub fn init_setup(&mut self) {

        debug!("init_setup {}", self.setup.name);

        let all = self.setup.buttons.clone();
        for i in 0..all.len() {
            self.init_button(i+1, &all[i]);
        }
    }

    pub fn init_button(&mut self, index: usize, btn: &BtnRef) -> Result<()> {

        debug!("init_button {}", btn.id);
        {
            self.button_mut(btn)?.index = Some(index);
        }

        self.decorate_button(btn)

    }


    pub fn decorate_button(&mut self, btn: &BtnRef) -> Result<()> {
        let mut device = self.device.borrow_mut();
        device.decorate_button(self.button(btn)?)?;
        Ok(())
    }


    fn ref_at(&self, index: usize) -> Option<BtnRef> {
        self.setup.buttons.get(index).cloned()
    }

    fn button<R: AsRef<BtnRef>>(&self, r: R) -> Result<&Button> {
        self.arena.get(r.as_ref().id).ok_or(DeckError::InvalidRef)
    }

    fn button_mut<R: AsRef<BtnRef>>(&mut self, r: R) -> Result<&mut Button> {
        self.arena.get_mut(r.as_ref().id).ok_or(DeckError::InvalidRef)
    }



    // fn new<T: ButtonDevice + 'a>(device: T) -> Self {
    //     ButtonDeck {
    //         device: Box::new(device)
    //     }
    // }

    fn wait_for_events(&self, timeout: usize) -> Result<Vec<DeckEvent>> {
        self.device.borrow_mut().wait_for_events(timeout)
    }

    fn on_button_down(&mut self, index: usize) -> Result<()> {
        debug!("on_button_down #{}", index);
        
        if let Some(br) = self.ref_at(index) {
            
            if let Some(bdf) = self.button(&br)?.effective_button_down() {
                bdf.call_fn(self,&br);
            }

            
            let switched = self.button_mut(&br)?.switch_state_action();
            if switched {
                self.decorate_button(&br)?;
            }


            let setup = match self.button_mut(&br)?.effective_switch_deck_setup() {
                Some(s) => Some(String::from(s)),
                None => None
            };

            if let Some(s) = setup {
                self.switch_to(&s);
            }

        }

       
        Ok(())
    }

    fn on_button_up(&mut self, index: usize) -> Result<()> {
        debug!("on_button_up #{}", index);
        if let Some(br) = self.ref_at(index) {
            if let Some(bdf) = self.button(&br)?.effective_button_up() {
                bdf.call_fn(self,&br);
            }
        }
        Ok(())
    }



}




pub fn button_deck_loop(deck: &mut ButtonDeck) -> Result<()>
{

    // deck.default_setup();

    loop {

        let events = deck.wait_for_events(100)?;
        for event in events {
            let result = match event {
                DeckEvent::ButtonUp(index) => {
                    deck.on_button_up(index)
                },
                DeckEvent::ButtonDown(index) => {
                    deck.on_button_down(index)
                },
            };
            // FIXME handle result
        }
    }
}


