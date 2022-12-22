use std::{path::{PathBuf, Path}, str::FromStr};

use crate::{device::PhysicalKey, deck::{FnRef, SetupRef}, DeckError};

type Result<T> = std::result::Result<T,DeckError>;

#[derive(Clone,Debug)]
pub enum StateRef2 {
    Id(usize,usize),
    Name(String),

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


#[derive(Clone,Debug)]
pub enum ButtonRef {
    // owner, index
    Id(usize,usize),
    Name(String)
}

impl ButtonRef {
    pub fn id(&self) -> Result<usize> {
        match self {
            ButtonRef::Id(_, id) => Ok(*id),
            ButtonRef::Name(_) => Err(DeckError::InvalidRef),
        }
    }
}

impl From<&str> for ButtonRef {
    fn from(s: &str) -> Self {
        ButtonRef::Name(String::from(s))
    }
}

impl AsRef<ButtonRef> for ButtonRef {
    fn as_ref(&self) -> &ButtonRef {
        self
    }
}


// impl <'a> AsRef<ButtonRef<'a>> for String {
//     fn as_ref(&self) -> ButtonRef<'a> {
//         ButtonRef::Name(&self)
//     }
// }

// #[derive(Clone,Debug)]
// pub struct ButtonRef {
//     pub (crate) id: usize,
//     pub (crate) state: Option<StateRef>
// }

// impl ButtonRef {
//     pub (crate) fn clone_with_state(&self, state: Option<StateRef>) -> Self {
//         ButtonRef { 
//             id: self.id,
//             state
//         }
//     }
// }

// impl AsRef<ButtonRef> for ButtonRef {
//     fn as_ref(&self) -> &ButtonRef {
//         self
//     }
// }


pub struct Button
{
    // pub (crate) newrwf: ButtonRef,

    // a private, unique id
    pub (crate) reference:  ButtonRef,

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

        self.switch_state(&next)
    }

    pub fn current_state<'a>(&'a self) -> &'a ButtonState {

        let cs = self.states.get(self.current_state)
            .expect("current_state must always be present");

        &cs.1

//        self.states.get(self.current_state).unwrap_or_else(|| self.states.get(self.default_state).expect("must not go wrong") )
    }

    pub fn get_state_ref(&self, name: &str) -> Option<StateRef> {
        self.states.iter().find(|(n,s)| n == name)
            .map(|(_,s)| s.reference.clone())
    }



    pub fn switch_state(&mut self, next_state: &StateRef) -> bool 
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

    pub (crate) reference: StateRef,

    pub (crate) color: Option<ButtonColor>,
    pub (crate) image: Option<ButtonImage>,

    pub (crate) on_button_down: Option<FnRef>,
    pub (crate) on_button_up:   Option<FnRef>,

    pub (crate) switch_button_state: Option<StateRef>,
    pub (crate) switch_deck_setup: Option<SetupRef>,

}



