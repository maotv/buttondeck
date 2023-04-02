use std::{path::{PathBuf, Path}, str::FromStr};

use log::{warn, debug, info};
use serde_json::Value;
use crate::SetupId;
use crate::{device::PhysicalKey, deck::{FnRef}, DeckError, ButtonId, StateId};

type Result<T> = std::result::Result<T,DeckError>;

const VALUE_NONE: ButtonValue = ButtonValue::None;




// #[derive(Clone,Debug)]
// pub enum StateRef2 {
//     Id(ButtonId,usize),
//     Name(String)
// }

// impl From<&str> for StateRef2 {
//     fn from(s: &str) -> Self {
//         StateRef2::Name(String::from(s))
//     }
// }


// impl AsRef<StateRef2> for StateRef2 {
//     fn as_ref(&self) -> &StateRef2 {
//         self
//     }
// }




pub struct Button
{
    // a private, unique id
    pub (crate) id:  ButtonId,

    pub (crate) name:  String,
    pub (crate) label: String,

    pub (crate) physical: Option<PhysicalKey>,

    pub (crate) defaults: ButtonState,

    pub (crate) default_state: StateId,
    pub (crate) current_state: StateId,

    pub (crate) states: Vec<ButtonState>,

}

impl Button {

    pub fn assigned_key(&self) -> Option<PhysicalKey> {
        self.physical.clone()
    }


    pub fn get_state_id(&self, name: &str) -> Option<StateId> {
        self.states.iter().find_map(|s| {
            if name == s.name {
                Some(s.id)
            } else {
                None
            }
        })
    }

    pub fn state<'a>(&'a self, id: StateId) -> Option<&'a ButtonState> {
        self.states.get(id.index)
    }

    pub fn state_mut<'a>(&'a mut self, id: StateId) -> Option<&'a mut ButtonState> {
        self.states.get_mut(id.index)
    }

    pub fn state_by_name_mut<'a>(&'a mut self, name: &str) -> Option<&'a mut ButtonState> {
        self.get_state_id(name).and_then(|id| self.state_mut(id))
    }

    pub fn current_state<'a>(&'a self) -> &'a ButtonState {
        self.state(self.current_state)
            .expect("current_state must always be present")
    }

    pub fn toggle_state(&mut self) -> bool {
        let next = if self.current_state.index == 0 {
            StateId { button: self.id, index: 1 }
        } else {
            StateId { button: self.id, index: 0 }
        };

        self.switch_state_internal(Some(next))
    }

    pub fn switch_state2(&mut self, next_state: StateId) -> bool 
    {
        self.switch_state_internal(Some(next_state))
    }

    pub fn switch_state_by_name(&mut self, next_state: &str) -> bool {
        self.switch_state_internal(self.get_state_id(next_state))
    }


    fn switch_state_internal(&mut self, next_state: Option<StateId>) -> bool 
    {

        info!("switch_state_internal: {:?}", next_state);
        let nextid = if let Some(id) = next_state {
            match self.state(id) {
                Some(s) => Some(s.id.clone()),
                None => None
            }
        } else {
            None
        };

        if let Some(id) = nextid {
            let last = self.current_state.clone();
            self.current_state = id;
    
            id.index != last.index
        } else {
            false
        }
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







    // fn switch_state_internal(&mut self, opt_next: Option<usize>) -> bool {

    //     if let Some(next) = opt_next {
            
    //         let last = self.current_state;
    //         self.current_state = next;
    
    //         next != last
    //     } else {
    //         false
    //     }
    // }



    pub fn switch_state_action(&mut self) -> bool {
        
        let b = match self.effective_switch_button_state() {
            Some(s) => Some(s.clone()),
            None => None
        };

        self.switch_state_internal(b)

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

    pub fn effective_switch_button_state<'a>(&'a self) -> Option<&'a StateId> {
        match &self.current_state().switch_button_state {
            Some(c) => Some(c),
            None => match &self.defaults.switch_button_state {
                Some(c) => Some(c),
                None => None
            }
        }
    }

    pub fn effective_switch_deck_setup<'a>(&'a self) -> Option<&'a SetupId> {
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

impl From<String> for ButtonValue {
    fn from(s: String) -> Self {
        ButtonValue::String(s)
    }
}

impl From<&str> for ButtonValue {
    fn from(s: &str) -> Self {
        ButtonValue::String(String::from(s))
    }
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



#[derive(Debug)]
pub struct ButtonState
{

    pub (crate) id: StateId,
    pub (crate) name: String,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) value: ButtonValue,


    pub (crate) on_button_down: Option<FnRef>,
    pub (crate) on_button_up:   Option<FnRef>,

    pub (crate) switch_button_state: Option<StateId>,
    pub (crate) switch_deck_setup: Option<SetupId>,

}



