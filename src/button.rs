use std::{path::{PathBuf, Path}, str::FromStr};

use log::warn;
use serde_json::Value;

use crate::{device::PhysicalKey, deck::{FnRef, SetupRef}, DeckError};

type Result<T> = std::result::Result<T,DeckError>;

const VALUE_NONE: ButtonValue = ButtonValue::None;

#[derive(Clone,Debug)]
pub enum StateRef2 {
    Id(usize,usize),
    Name(String)
}

impl From<&str> for StateRef2 {
    fn from(s: &str) -> Self {
        StateRef2::Name(String::from(s))
    }
}


impl AsRef<StateRef2> for StateRef2 {
    fn as_ref(&self) -> &StateRef2 {
        self
    }
}



#[derive(Clone,Debug)]
pub struct StateRef {
    pub id: usize,
    pub name: String
}

impl Default for StateRef {
    fn default() -> Self {
        Self { id: 0, name: String::from("default") }
    }
}


#[derive(Clone,Copy,Debug)]
pub struct ButtonId {
    owner: usize,
    index: usize
}



impl ButtonId {
    pub fn new(owner: usize, index: usize) -> Self {
        ButtonId { owner, index }
    }

    pub fn id(&self) -> usize {
        self.index
    }
}


pub struct Button
{
    // a private, unique id
    pub (crate) reference:  usize,

    pub (crate) name:  String,
    pub (crate) label: String,

    pub (crate) physical: Option<PhysicalKey>,

    pub (crate) defaults: ButtonState,

    pub (crate) default_state: usize,
    pub (crate) current_state: usize,

    pub (crate) states: Vec<(String,ButtonState)>,

}

impl Button {

    pub fn assigned_key(&self) -> Option<PhysicalKey> {
        self.physical.clone()
    }

    // FIXME. use stateref, not usize for current_state
    pub fn toggle_state(&mut self) -> bool {
        let next = if self.current_state == 0 {
            StateRef { id: 1, name: String::new() }
        } else {
            StateRef { id: 0, name: String::new() }
        };

        self.switch_state_xxx(&next)
    }

    pub fn current_state<'a>(&'a self) -> &'a ButtonState {

        let cs = self.states.get(self.current_state)
            .expect("current_state must always be present");

        &cs.1

    }


    pub fn state_by_ref_mut<'a>(&'a mut self, sref: StateRef2) -> Option<&'a mut ButtonState> {

        match sref {
            StateRef2::Id(bid, sid) => {
//                 if bid != self.reference { return None }
                match self.states.get_mut(sid) {
                    Some(s) => Some(&mut s.1),
                    None => None
                }
            },
            StateRef2::Name(name) => {
                let x = self.states.iter_mut().enumerate()
                    .find(|(i,(n,s))| n == &name)
                    .map(|(i, (n,s))| s);

                x
            }
        }

    }


    pub fn state_by_name_mut<'a>(&'a mut self, name: &str) -> Option<&'a mut ButtonState> {
        let bid = self.reference;
        self.states.iter_mut()
            .find(|(n,s)| n == name)
            .map(|(i,s)| s)
    }


    pub fn state_by_id<'a>(&'a self, id: usize) -> Option<&'a ButtonState> {

        match self.states.get(id) {
            Some(s) => Some(&s.1),
            None => None
        }

    }



    // pub fn get_state_ref(&self, name: &str) -> Option<StateRef> {
    //     self.states.iter().find(|(n,s)| n == name)
    //         .map(|(_,s)| s.reference.clone())
    // }

    pub fn get_state_ref2(&self, name: &str) -> Option<StateRef2> {
        let bid = self.reference;
        self.states.iter().enumerate().find(|(i,(n,s))| n == name)
            .map(|(i,s)| StateRef2::Id(bid, i))
    }

    pub fn set_state_image(&mut self, state_name: &str, icon: Option<ButtonImage>) -> Result<()> {


        if let Some(s) = self.state_by_name_mut(state_name) {
            s.image = icon;
        }

        Ok(())
    }

    pub fn set_state_value(&mut self, state_name: &str, value: ButtonValue) -> Result<()> {

        if let Some(s) = self.state_by_name_mut(state_name) {
            s.value = value
        }

        Ok(())

    }




    pub fn switch_state(&mut self, next_state: &str) -> bool {
        if let Some(s) = self.get_state_ref2(next_state) {
            self.switch_state2(&s)
        } else {
            false
        }
    }


    pub fn switch_state_xxx(&mut self, next_state: &StateRef) -> bool 
    {

        let next = match self.states.get(next_state.id) {
            Some(s) => next_state.id,
            None => self.current_state
        };
    
        let last = self.current_state;
        self.current_state = next;

        next != last

    }

    pub fn switch_state2(&mut self, next_state: &StateRef2) -> bool 
    {
        let opt_next = match next_state {
            StateRef2::Id(_, id) => {
                Some(*id)
            },
            StateRef2::Name(name) => {
                let r = self.states.iter()
                    .find(|(n,s) | name==n )
                    .map(|(n,r)| r.reference.id);
                    r

            },
        };


        // let next = match self.states.get(next_state.id) {
        //     Some(s) => next_state.id,
        //     None => self.current_state
        // };

        if let Some(next) = opt_next {
            
            let last = self.current_state;
            self.current_state = next;
    
            next != last
        } else {
            false
        }


    }


    pub fn switch_state_action(&mut self) -> bool {
        
        let b = match self.effective_switch_button_state() {
            Some(s) => Some(s.clone()),
            None => None
        };

        if let Some(sn) = b {
            self.switch_state_xxx(&sn)
        } else {
            false
        }
    }

    pub fn effective_value<'a>(&'a self) -> &'a ButtonValue {

        warn!("Button Value is {:?} {:?}", &self.current_state().value, &self.defaults.value);
        
        match &self.current_state().value {
            ButtonValue::None => {
                let dv = &self.defaults.value;
                match dv {
                    ButtonValue::None => &VALUE_NONE,
                    _ => self.get_true_button_value(dv)
                }
            }
            V => {
                self.get_true_button_value(V)
            }
        }

        // match cv {
        //     ButtonValue::None => {
        //         let dv = &self.defaults.value;
        //         match dv {
        //             ButtonValue::None => dv,
                    
        //         }

        //     }
            
            
            
        //     match &self.defaults.value {
        //         ButtonValue::None => ButtonValue::None
        //         Some(c) => Some(c),
        //     },
        //     x => x 
        // }
    }

    fn get_true_button_value<'a>(&'a self, bv: &'a ButtonValue) -> &'a ButtonValue {
        // TODO handle Rotary value, other values here
        bv
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

    pub fn effective_button_down<'a>(&'a self) -> Option<&'a FnRef> {
        match &self.current_state().on_button_down {
            Some(c) => Some(c),
            None => match &self.defaults.on_button_down {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_button_up<'a>(&'a self) -> Option<&'a FnRef> {
        match &self.current_state().on_button_up {
            Some(c) => Some(&c),
            None => match &self.defaults.on_button_up {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_switch_button_state<'a>(&'a self) -> Option<&'a StateRef> {
        match &self.current_state().switch_button_state {
            Some(c) => Some(c),
            None => match &self.defaults.switch_button_state {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_switch_deck_setup<'a>(&'a self) -> Option<&'a SetupRef> {
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
    pub fn from_path(s: &str) -> Option<Self> {

        let p = PathBuf::from(s);
        if p.exists() {
            Some(ButtonImage {
                path: PathBuf::from(s)
            })
        } else {
            None
        }
    }


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

#[derive(Debug, Clone)]
pub enum ButtonValue {

    None,
    Bool(bool),
    OnOff,
    String(String),
    Number,

    Error(String)
}

impl From<Value> for ButtonValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => ButtonValue::None,
            Value::Bool(b) => ButtonValue::Bool(b),
            Value::Number(n) => {
                ButtonValue::None
            },
            Value::String(s) => ButtonValue::String(s),
            Value::Array(_) => ButtonValue::Error(String::from("Arrays are not supported in ButtonValue")),
            Value::Object(_) => ButtonValue::Error(String::from("Objects are not supported in ButtonValue")),
        }
    }
}

impl ToString for ButtonValue {
    fn to_string(&self) -> String {
        match self {
            ButtonValue::None => String::new(),
            ButtonValue::Bool(b) => b.to_string(),
            ButtonValue::OnOff => String::from("on/off"),
            ButtonValue::String(s) => s.clone(),
            ButtonValue::Number => String::from("TODO"),
            ButtonValue::Error(e) => format!("Error: {:?}", e),
        }
    }
}


impl Default for ButtonValue {
    fn default() -> Self {
        ButtonValue::None
    }
}




#[derive(Default)]
pub struct ButtonState
{

    pub (crate) reference: StateRef,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) value: ButtonValue,


    pub (crate) on_button_down: Option<FnRef>,
    pub (crate) on_button_up:   Option<FnRef>,

    pub (crate) switch_button_state: Option<StateRef>,
    pub (crate) switch_deck_setup: Option<SetupRef>,

}



