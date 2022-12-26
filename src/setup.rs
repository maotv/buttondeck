use std::{fs::File, sync::Arc, collections::HashMap, rc::Rc, cell::RefCell, path::{PathBuf, Path}};

use hidapi::HidApi;
use indexmap::IndexMap;
use serde_derive::{Serialize,Deserialize};

use crate::{Button, ButtonSetup, ButtonState, ButtonColor, deck::{ButtonMapping, FnRef, SetupRef, FnArg}, device::{PhysicalKey, ButtonDevice, DeviceEvent}, DeviceFamily, DeviceKind, ButtonDeviceTrait, DeckEvent, button::{StateRef, ButtonImage}, StateRef2, ButtonId};

use super::{DeckError, ButtonDeck, device::StreamDeckDevice, ButtonFn};

use log::{error, debug, warn, info, trace};

type Result<T> = std::result::Result<T,DeckError>;

#[derive(Default,Serialize,Deserialize)]
struct DeckJson {
    home:     Option<String>,
    midi_in:  Option<String>,
    midi_out: Option<String>,
    deck: Option<ButtonDeckTemplate>,
}


#[derive(Serialize,Deserialize)]
struct ButtonDeckTemplate {
    label:    String,
    wiring:   IndexMap<String,PhysicalKeyTemplate>,
    controls: IndexMap<String,ButtonTemplate>,
    setups:   IndexMap<String,SetupTemplate>
}


#[derive(Serialize,Deserialize)]
pub struct PhysicalKeyTemplate {
    id:   usize,
}

impl PhysicalKeyTemplate {
    pub fn into_key(&self, name: &str) -> Result<PhysicalKey> {
        Ok(PhysicalKey {
            id: self.id,
            name: String::from(name),
        })
    }
}



#[derive(Serialize,Deserialize)]
struct ButtonTemplate {

    label: Option<String>,
    color: Option<String>,
    image: Option<String>,
    on_up: Option<String>,
    on_down:  Option<String>,
    on_value: Option<String>,

    switch_button_state: Option<String>,
    switch_deck_setup: Option<String>,

    states: Option<IndexMap<String,StateTemplate>>
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
    mapping: HashMap<String,ReferenceTemplate>
}

#[derive(Serialize,Deserialize)]
struct ReferenceTemplate {
    control: String,
    state:   Option<String>
}



#[derive(Default)]
pub struct ButtonDeckBuilder {
    kind: DeviceKind,
    pwd: PathBuf,
    hidapi: Option<HidApi>,
    config: Option<PathBuf>,
    home: Option<PathBuf>,
    midi_in: Option<String>,
    midi_out: Option<String>,
    functions: RefCell<Vec<(String,ButtonFn)>>,
}

impl ButtonDeckBuilder {

    pub fn new(kind: DeviceKind) -> Self {
        ButtonDeckBuilder {
            kind,
            ..Default::default()
        }
    }

    pub fn home_path<'a>(&'a self) -> &'a Path {
        match &self.home {
            Some(p) => p.as_ref(),
            None => self.pwd.as_ref()
        }
    }

    pub fn with_config(&mut self, config: &str) -> &mut Self {
        self.config  = Some(PathBuf::from(config));
        let op = PathBuf::from(config).parent().map(|p| PathBuf::from(p));
        self.pwd = op.unwrap_or_else(|| PathBuf::default());
        self
    }

    pub fn with_functions(&mut self, mut functions: Vec<(String,ButtonFn)>) -> &mut Self {

        self.functions.borrow_mut().append(&mut functions);
        self
    }

    pub fn with_function<F>(&mut self, name: &str, function: F) -> &mut Self 
        where F: FnMut(&mut ButtonDeck, FnArg) -> Result<()> + Send + 'static
    {
        {
            let bf = ButtonFn { func: Box::new(function) };
            let mut fa = self.functions.borrow_mut();
            fa.push((String::from(name),bf));
        }


        self
    }

    


    pub fn with_hidapi(&mut self, hidapi: HidApi) -> &mut Self {
        self
    }


    // pub fn get_button_fn<'a>(&'a self, name: &Option<String>) -> Option<&'a ButtonFn> {
    //     match name  {
    //         Some(s) => self.functions.iter().find(|f| f.name() == s ),
    //         None => None
    //     }
    // }


    pub fn build(&mut self) -> Result<ButtonDeck> {

        trace!("Build from config: {:?}", &self.config);



        let deckjson: DeckJson = match &self.config {
            Some(c) => serde_json::from_reader(File::open(c)?)?,
            None => DeckJson::default()
        };

        if let Some(s) = deckjson.midi_in {
            self.midi_in = Some(s)
        }

        if let Some(s) = deckjson.midi_out {
            self.midi_out = Some(s)
        }


        let specs = self.kind.get_specs();


        trace!("family is {:?}",specs.family);
        let device = match specs.family {

            DeviceFamily::Streamdeck => {
                let mut hidapi = match self.hidapi.take() {
                    Some(api) => api,
                    None => HidApi::new()?
                };
                crate::device::open_streamdeck(&mut hidapi, self.kind)
            }

            DeviceFamily::Midi => {
                crate::device::open_midi(self.kind, self.midi_in.clone(), self.midi_out.clone())
            }

        }?;

        let mut deck = build_buttondeck(self, deckjson.deck, device)?;
        deck.initialize()?;
        // deck.switch_to_name("default");

        Ok(deck)
//        Err(DeckError::Message(String::from("NYI")))

    }


}




struct BuilderData<'a> {
    builder:     &'a ButtonDeckBuilder,
    setup_refs:  Vec<Prep<'a,SetupRef,SetupTemplate>>,
    button_refs: Vec<Prep<'a,ButtonId,ButtonTemplate>>,
    function_refs: Vec<FnRef>,
}

impl<'a> BuilderData<'a> {

    fn setup_for_opt_name(&self, name: &Option<String>) -> Option<SetupRef> {
        match name {
            Some(s) => {
                self.setup_refs.iter().map(|s| s.reference.clone()).find(|r| r.name == s.as_str()).clone()
            },
            None => None         
        }
    }


    pub fn get_button_fn_ref(&'a self, name: &Option<String>) -> Option<&'a FnRef> {
        match name  {
            Some(s) => self.function_refs.iter().find(|f| &f.name == s ),
            None => None
        }
    }


}


struct Prep<'a,R,T> {
    name: &'a str,
    reference: R,
    template: &'a T
}



fn build_buttondeck(builder: &ButtonDeckBuilder, opt_template: Option<ButtonDeckTemplate>, any_device: ButtonDevice /* , functions: Vec<ButtonFn>, path: P */)  -> Result<ButtonDeck> {

    let deckid = 42;

    // debug!("setup::build_buttondeck {:?} with dir {:?}", &device.model(), home_folder);
    let device: &dyn ButtonDeviceTrait = match &any_device {
        ButtonDevice::Streamdeck(sd) => sd as &dyn ButtonDeviceTrait,
        ButtonDevice::Midi(md) => md,
    };


    let template = match opt_template {
        Some(t) => t,
        None => {
            let json_path = builder.home_path().join(format!("{}.json", device.model()));
            trace!("Reading config from {:?}", json_path);
        
            let f = File::open(json_path)?;
            let xt: ButtonDeckTemplate = serde_json::from_reader(f)?;
            trace!("Done reading config");
            xt
        }
    };


    let maxid = template.wiring.iter().map(|(n,w)| w.id).max().unwrap_or(127);
    trace!("Max button id is {}", maxid);

    let mut phys: Vec<Option<PhysicalKey>> = vec![None;maxid+1];
    let mut phymap: HashMap<String,PhysicalKey> = HashMap::new();

    for (n,pt) in &template.wiring {
        let p: PhysicalKey = pt.into_key(n)?;
        if phys[p.id].is_some() { return  Err(DeckError::Message(format!("duplicate id: {}", p.id)));} 
        // if phymap.contains_key(&p.name) { return  Err(DeckError::Message(format!("duplicate name: {}", p.name))); }

        trace!("Physical Key: {:?}", p);
        phys[p.id] = Some(p.clone());
        phymap.insert(n.clone(), p);
    }

    let setup_refs: Vec<Prep<SetupRef,SetupTemplate>> = template.setups.iter().enumerate()
        .map(|(i,(n,t))| Prep {
            name: n,
            reference: SetupRef { id: i, name: n.clone() } ,
            template: t,
        })
        .collect();

    let button_refs: Vec<Prep<ButtonId,ButtonTemplate>> = template.controls.iter().enumerate()
        .map(|(i,(n,t))| {
            Prep {
                name: n,
                reference: ButtonId::new(deckid, i), //  { id: i, state: None },
                template:t,
            }
        })
        .collect();

    let functionvec: Vec<(String,Rc<RefCell<ButtonFn>>)> = builder.functions.take().into_iter()
        .map(|(n,f)| (n, Rc::new(RefCell::new(f))))
        .collect();

    let function_refs: Vec<FnRef> = functionvec.iter().enumerate()
        .map(|(i,(n,f))| FnRef{ id: i, name: String::from(n) })
        .collect();

//    let function_refs: Vec<NamedRef> = 


    let mut data = BuilderData {
        builder,
        setup_refs,
        button_refs,
        function_refs
    };


    let button_arena: Vec<Button> = data.button_refs.iter().enumerate()
        .filter_map(|(i,p)| build_button(&data, i).ok())
        .collect();


    // this is where all buttons live
    // let arena: Vec<Button> = template.controls.iter()
    //     .map(|(n,b)| build_button(&data, n, b)).collect();

    let button_map: HashMap<String,ButtonId> = button_arena.iter().enumerate()
        .map(|(i,b)| (b.name.clone(), ButtonId::new(deckid, i)) ).collect();

//    let setup_map: HashMap<String,ButtonSetup> = HashMap::new();
    let mut setup_arena: Vec<ButtonSetup> = Vec::new(); // HashMap::new();
        // template.setups.iter()
    //     .map(|(n,t)| (String::from(n),build_button_setup(&button_map, n,t)) )
    //     .collect();



//    for (sn,st) in template.setups.iter() {
    for prep in data.setup_refs.iter() {

        trace!("  Setup {}", prep.name);

        let st = prep.template;

        // let mut controls: HashMap<PhysicalKey,ButtonRef> = HashMap::new();
        let mut mapping = Vec::new();

        for (pn,rt) in &st.mapping {

            let b = button_map.get(&rt.control);
            let p = phymap.get(pn).clone();
            let s = 

            if let Some(br) = b {

                if let Some(button) = button_arena.get(br.id()) {

                    let s = rt.state.clone().and_then(|n| button.get_state_ref(&n) );
                    let s2 = s.map(|x| StateRef2::Id(0, x.id));
                    // let s = button.get_state_ref(&rt.state);

                    if let Some(pk) = p {
                        mapping.push(ButtonMapping { 
                            key: pk.clone(), 
                            button: br.clone(),
                            state: s2
                        })
                    }

                }

            };

        }
    
        // setup_map.insert(sn.clone(), ButtonSetup { name: sn.clone(), mapping });
        setup_arena.push(ButtonSetup { reference: prep.reference.clone(),  mapping});

    }


    let ccm: Vec<Option<ButtonMapping>> = phys.iter().map(|_| None).collect();


    // let xdev: Rc<RefCell<Box<dyn ButtonDeviceTrait>>> = match any_device {
    //     ButtonDevice::Streamdeck(sd) => Rc::new(RefCell::new(Box::new(sd))),
    //     ButtonDevice::Midi(md) => Rc::new(RefCell::new(Box::new(md))),
    // };

    // dummy channels
    let (tx,rx) = std::sync::mpsc::channel::<DeviceEvent>(); 
    let (bdtx,bdrx) = std::sync::mpsc::channel::<DeckEvent>(); 

    Ok(ButtonDeck {

        deckid: 42,

        device: Some(any_device),
        // device: xdev, // Rc::new(RefCell::new(Box::new(device))),
        folder: PathBuf::from(builder.home_path()),
        wiring: phys,
        functions: functionvec,
        current_key_map: ccm,
        button_arena,
//         button_map,
        current_setup: 0,
        setup_arena,
        self_sender: bdtx,
        device_sender: tx,
        device_receiver: Some(bdrx),
        // deckid: todo!(),
    })

}


fn state_for_opt_name(data: &Vec<Prep<StateRef,StateTemplate>>, name: &Option<String>) -> Option<StateRef> {
    match name {
        Some(s) => {
            data.iter().map(|s| s.reference.clone()).find(|r| r.name == s.as_str()).clone()
        },
        None => None         
    }
    
}   

// fn build_button(data: &BuilderData, n: &str, bt: &ButtonTemplate) -> Button {
fn build_button(data: &BuilderData, index: usize) -> Result<Button> {

    let prep = data.button_refs.get(index).ok_or_else(|| DeckError::Message(String::from("Internal Error(build_button#1)")))?;

    let n = prep.name;
    let bt = prep.template;

    trace!("Build Button: {} -> {:?}", n, bt.label);

    let empty_map = IndexMap::new();
    let state_templates = &bt.states.as_ref().unwrap_or(&empty_map);

    let state_prep: Vec<Prep<StateRef,StateTemplate>> = state_templates.iter().enumerate()
        .map(|(i,(n,t))| Prep {
            name: n,
            reference: StateRef { id: i, name: String::from(n) } ,
            template: t,
        }).collect();


    let defaults = ButtonState {
        // name: String::from(""),
        reference: StateRef { name: String::from("default"), id: 0 }, 
        color: ButtonColor::from_option_string(&bt.color), 
        image: ButtonImage::from_option_string(&data.builder.home_path(), &bt.image),

        on_button_down: data.get_button_fn_ref(&bt.on_down).cloned(), 
        on_button_up: data.get_button_fn_ref(&bt.on_up).cloned(), 
        
        switch_button_state: state_for_opt_name(&state_prep, &bt.switch_button_state),
        switch_deck_setup: data.setup_for_opt_name(&bt.switch_deck_setup),
    };
    



    // let bsfinal: Vec<ButtonState> = Vec::new();
    // let bsref = bsfinal.as_ptr() as usize;

    let states: Vec<(String,ButtonState)> = if state_prep.is_empty() {
        vec! [ (String::from("default"), ButtonState::default()) ]
    } else {
        state_prep.iter()
            .map(|p| {
                (String::from(p.name), ButtonState { 
                    reference: p.reference.clone(),
                    // name: String::from(n),
                    color: ButtonColor::from_option_string(&p.template.color), 
                    image: ButtonImage::from_option_string(data.builder.home_path(), &p.template.image), 
                    on_button_down: data.get_button_fn_ref(&p.template.on_down).cloned(), 
                    on_button_up: data.get_button_fn_ref(&p.template.on_up).cloned(),
                    switch_button_state: state_for_opt_name(&state_prep, &p.template.switch_button_state), //  s.switch_button_state.clone(),
                    switch_deck_setup: data.setup_for_opt_name(&p.template.switch_deck_setup),
                })
            })
            .collect()
    };

    Ok(Button {
        // button unique id
        reference: prep.reference.id(),
        
        name: String::from(n),
        label: bt.label.clone().unwrap_or_else(|| String::from(n)),
        physical: None,

        default_state: 0,
        current_state: 0,

        states,
        defaults
    })
    

}

