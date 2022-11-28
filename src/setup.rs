use std::{fs::File, sync::Arc, collections::HashMap, rc::Rc, cell::RefCell, path::{PathBuf, Path}};

use hidapi::HidApi;
use serde_derive::{Serialize,Deserialize};

use crate::{Button, ButtonSetup, ButtonState, ButtonColor, deck::ButtonImage};

use super::{DeckError, ButtonDeck, BtnRef, device::StreamDeckDevice, ButtonFn};

use log::{error, debug, warn, info};

type Result<T> = std::result::Result<T,DeckError>;


#[derive(Serialize,Deserialize)]
struct DeckJson {
    home: Option<String>
}




#[derive(Serialize,Deserialize)]
struct ButtonDeckTemplate {
    buttons: HashMap<String,ButtonTemplate>,
    setups: HashMap<String,SetupTemplate>
}


#[derive(Serialize,Deserialize)]
struct ButtonTemplate {

    label: Option<String>,
    color: Option<String>,
    image: Option<String>,
    on_up: Option<String>,
    on_down: Option<String>,
    on_value: Option<String>,

    switch_button_state: Option<String>,
    switch_deck_setup: Option<String>,

    states: Option<HashMap<String,StateTemplate>>
}

#[derive(Serialize,Deserialize)]
struct StateTemplate {

    color: Option<String>,
    image: Option<String>,
    effect: Option<String>,

    on_up: Option<String>,
    on_down: Option<String>,
    on_value: Option<String>,

    switch_button_state: Option<String>,
    switch_deck_setup: Option<String>,
}

#[derive(Serialize,Deserialize)]
struct SetupTemplate {
    label: Option<String>,
    buttons: Vec<ReferenceTemplate>
}

#[derive(Serialize,Deserialize)]
struct ReferenceTemplate {
    button: String,
    state:  Option<String>
}





#[derive(Default)]
pub struct ButtonDeckBuilder {
    pwd: PathBuf,
    hidapi: Option<HidApi>,
    config: Option<PathBuf>,
    home: Option<PathBuf>,
    functions: HashMap<String,ButtonFn>,
}

impl ButtonDeckBuilder {

    pub fn new() -> Self {
        ButtonDeckBuilder::default()
    }

    pub fn home_path<'a>(&'a self) -> &'a Path {
        match &self.home {
            Some(p) => p.as_ref(),
            None => self.pwd.as_ref()
        }
    }

    pub fn with_config(&mut self, config: &str) -> &mut Self {
        self.config  = Some(PathBuf::from(config));
        self
    }

    pub fn with_functions(&mut self, functions: Vec<ButtonFn>) -> &mut Self {
        for b in functions.into_iter() {
            match &b {
                ButtonFn::NoArg(n, _) => self.functions.insert(String::from(n), b.clone()),
                ButtonFn::DeckArgs(n, _) => self.functions.insert(String::from(n), b.clone())
            };
        }

        self
    }




    pub fn with_hidapi(&mut self, hidapi: HidApi) -> &mut Self {
        self
    }


    pub fn get_button_fn<'a>(&'a self, name: &Option<String>) -> Option<&'a ButtonFn> {
        match name  {
            Some(s) => self.functions.get(s),
            None => None
        }
    }

    // pub fn build(&mut self) -> Result<Vec<ButtonDeck>> {
    //     Ok(vec!()) // much later
    // }

    pub fn build_first_streamdeck(&mut self) -> Result<ButtonDeck> {

        let config_path = self.config.clone().unwrap_or_else(|| PathBuf::from("./deck.json"));

        info!("Loading confing from {:?}", &config_path);
        let deckjson: DeckJson = serde_json::from_reader(File::open(&config_path)?)?;
    
        self.home = Some(match deckjson.home {
            Some(s) => Some(PathBuf::from(s)),
            None => config_path.parent().map(|p| PathBuf::from(p))
        }.ok_or(DeckError::NoDirectory)?);

        info!("Buttondeck home is {:?}", &self.home);

        let mut hidapi = match self.hidapi.take() {
            Some(api) => api,
            None => HidApi::new()?
        };

        let device = crate::device::open_first_streamdeck(&mut hidapi)?;
    
        let mut deck = build_buttondeck(self, device)?;
        deck.switch_to("default");
    
    
        Ok(deck)


    }

}





pub fn build_buttondeck(builder: &ButtonDeckBuilder, device: StreamDeckDevice /* , functions: Vec<ButtonFn>, path: P */)  -> Result<ButtonDeck> {


    // debug!("setup::build_buttondeck {:?} with dir {:?}", &device.model(), home_folder);

    let json_path = builder.home_path().join(format!("{}.json", device.model()));


    let f = File::open(json_path)?;
    let template: ButtonDeckTemplate = serde_json::from_reader(f)?;

    // println!("{}", serde_json::to_string_pretty(&template)?);
    // let x = build_button_deck(rt, template);

    let arena: Vec<Button> = template.buttons.iter()
        .map(|(n,b)| build_button(&builder, n, b)).collect();

    let button_map: HashMap<String,BtnRef> = arena.iter().enumerate()
        .map(|(i,b)| (b.name.clone(), BtnRef{ id: i, state: None }) ).collect();

    let setup_map: HashMap<String,ButtonSetup> = template.setups.iter()
        .map(|(n,t)| (String::from(n),build_button_setup(&button_map, n,t)) )
        .collect();

    // let mut setup_map: HashMap<String,ButtonSetup> = HashMap::new();


    let setup = match setup_map.get("default") {
        Some(bs) => bs.clone(),   
        None => {
            if ( setup_map.len() > 0 ) {
                match setup_map.iter().map(|(a,b)| b).nth(0) {
                    Some(bs) => bs.clone(),
                    None => ButtonSetup::default()
                }
            } else {
                ButtonSetup::default()
            }
        }
    };


    Ok(ButtonDeck {
        device: Rc::new(RefCell::new(Box::new(device))),
        folder: PathBuf::from(builder.home_path()),
        arena,
        button_map,
        setup,
        setup_map,
    })

}


fn build_button_setup(map: &HashMap<String,BtnRef>, name: &str, template: &SetupTemplate) -> ButtonSetup {

    debug!("build_button_setup {}", name);

    match try_build_button_setup(map,name,template) {
        Ok(s) => s,
        Err(e) => {
            error!("Error building button setup: {:?}", e);
            ButtonSetup {
                name: String::from(name),
                buttons: vec!()
            }
        }
    }
}


fn try_build_button_setup(map: &HashMap<String,BtnRef>, name: &str, template: &SetupTemplate) -> Result<ButtonSetup> {


    debug!("try_build_button_setup {}", name);

    let mut buttons: Vec<BtnRef> = vec!();

    for t in &template.buttons {
        let b = map.get(&t.button).ok_or_else(|| DeckError::InvalidRef)?;
        debug!("push {}", b.id);
        buttons.push(b.clone_with_state(t.state.clone()))
    }


    Ok(ButtonSetup {
        name: String::from(name),
        buttons
    })
}


fn build_button(builder: &ButtonDeckBuilder, n: &str, b: &ButtonTemplate) -> Button {

    println!("Build Button: {} -> {:?}", n, b.label);

    let states = match &b.states {
        Some(bs) => {
            bs.iter().map(|(n,s)| {

                ButtonState { 
                    name: String::from(n),
                    color: ButtonColor::from_option_string(&s.color), 
                    image: ButtonImage::from_option_string(builder.home_path(), &s.image), 
                    on_button_down: builder.get_button_fn(&s.on_down).cloned(), 
                    on_button_up: builder.get_button_fn(&s.on_up).cloned(),
                    switch_button_state: s.switch_button_state.clone(),
                    switch_deck_setup: s.switch_deck_setup.clone(),
                }
                
            }).collect()
        },
        None => {
            vec!()
        }
    };


    Button {
        name: String::from(n),
        label: b.label.clone().unwrap_or_else(|| String::from(n)),
        index: None,
        color: ButtonColor::from_option_string(&b.color), 
        image: ButtonImage::from_option_string(&builder.home_path(), &b.image), 
        on_button_down: builder.get_button_fn(&b.on_down).cloned(), 
        on_button_up: builder.get_button_fn(&b.on_up).cloned(), 
        switch_button_state: b.switch_button_state.clone(),
        switch_deck_setup: b.switch_deck_setup.clone(),
        current_state: 0,
        states, 
        default_state: ButtonState::default()
    }

}

