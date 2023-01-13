use std::{fs::File, sync::{Arc, atomic::{AtomicUsize, Ordering}}, collections::HashMap, rc::Rc, cell::RefCell, path::{PathBuf, Path}, thread::{self, JoinHandle}, time::Instant};

use hidapi::HidApi;
use indexmap::IndexMap;
use serde_derive::{Serialize,Deserialize};
use serde_json::Value;

use crate::{Button, ButtonSetup, ButtonState, ButtonColor, deck::{ButtonMapping, FnRef, SetupRef, FnArg, DeckDeviceSetup}, device::{PhysicalKey, ButtonDevice, DeviceEvent}, DeviceFamily, DeviceKind, ButtonDeviceTrait, DeckEvent, button::{StateRef, ButtonImage, ButtonValue}, StateRef2, ButtonId};

use super::{DeckError, ButtonDeck, device::StreamDeckDevice, ButtonFn};

use log::{error, debug, warn, info, trace};

type Result<T> = std::result::Result<T,DeckError>;


static idgen: AtomicUsize = AtomicUsize::new(1);

#[derive(Default,Serialize,Deserialize)]
struct DeckJson {

    home:     Option<String>,

    midi_in:  Option<String>,
    midi_out: Option<String>,

    devices:  Option<HashMap<String,ButtonDeckTemplate>>,

    controls: Option<IndexMap<String,ButtonTemplate>>,
    setups:   Option<IndexMap<String,SetupTemplate>>,


    deck: Option<ButtonDeckTemplate>,

}

#[derive(Serialize,Deserialize)]
struct ButtonDeckTemplate {
    label:    Option<String>,
    wiring:   IndexMap<String,PhysicalKeyTemplate>,
    controls: Option<IndexMap<String,ButtonTemplate>>,
    setups:   Option<IndexMap<String,SetupTemplate>>,
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

    #[serde(default)]
    value: Value,

    on_up: Option<String>,
    on_down:  Option<String>,
    on_value: Option<String>,

    switch_button_state: Option<String>,
    switch_deck_setup: Option<String>,

    states: Option<IndexMap<String,StateTemplate>>
}

#[derive(Serialize, Deserialize, Default)]
struct StateTemplate {

    color: Option<String>,
    image: Option<String>,
    effect: Option<String>,

    #[serde(default)]
    value: Value,

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
struct XCell<T> 
    where T: Default
{
    item: T
}

impl <T> XCell<T> 
    where T: Default
{
    pub fn borrow_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

// #[derive()]
pub struct ButtonDeckBuilder<D> 
    where D: 'static + Sync + Send 
{
    kind: DeviceKind,
    pwd: PathBuf,
    hidapi: Option<HidApi>,
    config: Option<PathBuf>,
    home: Option<PathBuf>,
    midi_in: Option<String>,
    midi_out: Option<String>,
    data: Option<D>,
    functions: Vec<(String,ButtonFn<D>)>,
}

impl <D> ButtonDeckBuilder<D> 
    where D: Sync + Send + 'static
{

    pub fn new(kind: DeviceKind) -> Self {
        ButtonDeckBuilder {
            kind,
            data: None,
            pwd: Default::default(),
            hidapi: None,
            config: None,
            home: None,
            midi_in: None,
            midi_out: None,
            functions: Vec::new(),
                }
    }

    pub fn home_path<'a>(&'a self) -> &'a Path {
        match &self.home {
            Some(p) => p.as_ref(),
            None => self.pwd.as_ref()
        }
    }

    pub fn with_config<P: AsRef<Path>>(mut self, config: P) -> Self {
        self.config  = Some(PathBuf::from(config.as_ref()));
        let op = PathBuf::from(config.as_ref()).parent().map(|p| PathBuf::from(p));
        self.pwd = op.unwrap_or_else(|| PathBuf::default());
        self
    }

    pub fn with_data(mut self, data: D) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_functions(mut self, mut functions: Vec<(String,ButtonFn<D>)>) -> Self {

        self.functions.append(&mut functions);
        self
    }

    pub fn with_function<F>(mut self, name: &str, function: F) -> Self 
        where F: FnMut(&mut ButtonDeck<D>, FnArg) -> Result<()> + Send + Sync + 'static
    {
        {
            let bf = ButtonFn { func: Box::new(function) };
            // let mut fa = self.functions;
            self.functions.push((String::from(name),bf));
        }


        self
    }

    


    pub fn with_hidapi(mut self, hidapi: HidApi) -> Self {
        self.hidapi = Some(hidapi);
        self
    }


    // pub fn get_button_fn<'a>(&'a self, name: &Option<String>) -> Option<&'a ButtonFn> {
    //     match name  {
    //         Some(s) => self.functions.iter().find(|f| f.name() == s ),
    //         None => None
    //     }
    // }

    


    pub fn spawn(mut self: ButtonDeckBuilder<D>) -> JoinHandle<()> {
        thread::spawn(move || {
            match self.build() {
                Ok(mut buttondeck) => {
                    buttondeck.run()
                },
                Err(e) => {
                    error!("Build Error: {:?}", e)
                }
            }
        })
    }


    pub fn build(mut self) -> Result<ButtonDeck<D>> {

            // collect all functions (rc<refcell<>>) as name,rc tuples in a vec
        let mut functionvec: Vec<(String,Rc<RefCell<ButtonFn<D>>>)> = Vec::new();
        for (n,f) in self.functions.drain(..) {
            functionvec.push((n, Rc::new(RefCell::new(f))))
        }

        // create references for the functions
        let function_refs: Vec<FnRef> = functionvec.iter().enumerate()
            .map(|(i,(n,f))| FnRef{ id: i, name: String::from(n) })
            .collect();

        // dummy channels
        let (dvtx,dvrx) = std::sync::mpsc::channel::<DeviceEvent>(); 
        let (bdtx,bdrx) = std::sync::mpsc::channel::<DeckEvent>();

        Ok(ButtonDeck {

            deckid: idgen.fetch_add(1, Ordering::SeqCst),

            hidapi: self.hidapi.take(),

            // device: xdev, // Rc::new(RefCell::new(Box::new(device))),
            folder: PathBuf::from(self.home_path()),
            ddsetup: Default::default(),
            functions: functionvec,
            
            // dummy!
            device_event_sender: dvtx,
            // dummy!
            deck_event_sender: bdtx,
            // dummy!
            deck_event_receiver: Some(bdrx), // TODO FIXME no option needed

            data: self.data.take(),

            other: None,
            builder: self,
        })

    }

    pub fn build_for_device(&mut self, device: ButtonDevice) -> Result<DeckDeviceSetup> {

        let deckjson: DeckJson = match &self.config {
            Some(c) => serde_json::from_reader(File::open(c)?)?,
            None => DeckJson::default()
        };


        build_buttondeck(self, deckjson, device)

    }


//     pub fn build_old(&mut self) -> Result<ButtonDeck<D>> {

//         trace!("Build from config: {:?}", &self.config);

//         let deckjson: DeckJson = match &self.config {
//             Some(c) => serde_json::from_reader(File::open(c)?)?,
//             None => DeckJson::default()
//         };

//         if let Some(s) = &deckjson.midi_in {
//             self.midi_in = Some(s.clone())
//         }

//         if let Some(s) = &deckjson.midi_out {
//             self.midi_out = Some(s.clone())
//         }


//         let specs = self.kind.get_specs();


//         let mut opt_hidapi = None; 

//         trace!("family is {:?}",specs.family);
//         let device = match specs.family {

//             DeviceFamily::Streamdeck => {
//                 let mut hidapi = match self.hidapi.take() {
//                     Some(api) => api,
//                     None => HidApi::new()?
//                 };

//                 let r = crate::device::open_streamdeck(&mut hidapi, self.kind);
//                 opt_hidapi = Some(hidapi);
//                 r
//             }

//             DeviceFamily::Midi => {
//                 crate::device::open_midi(self.kind, self.midi_in.clone(), self.midi_out.clone())
//             }

//         }?;

//         let mut deck = build_buttondeck(self, deckjson, device)?;

//         deck.hidapi = opt_hidapi;
//         deck.initialize()?;

//         // deck.switch_to_name("default");

//         Ok(deck)
// //        Err(DeckError::Message(String::from("NYI")))

//     }


}

// pub fn rebuild<D>(djraw: Value) -> Result<ButtonDeck<D>> {

//     let deckjson = serde_json::from_value::<DeckJson>(djraw)?;

//     let device: &dyn ButtonDeviceTrait = match &any_device {
//         ButtonDevice::Streamdeck(sd) => sd as &dyn ButtonDeviceTrait,
//         ButtonDevice::Midi(md) => md,
//     };


//     let model = device.model();

//     info!("build_buttondeck for device {}", model);



//     let mut deck = build_buttondeck(self, deckjson, device)?;




//     deck.hidapi = opt_hidapi;
//     deck.initialize()?;

//     // deck.switch_to_name("default");

//     Ok(deck)

// }


struct BuilderData<'a,D> 
    where D: Send + Sync + 'static
{
    builder:     &'a ButtonDeckBuilder<D>,
    setup_refs:  Vec<Prep<'a,SetupRef,SetupTemplate>>,
    button_refs: Vec<Prep<'a,ButtonId,ButtonTemplate>>,
    function_refs: Vec<FnRef>,
}

impl<'a,D> BuilderData<'a,D> 
    where D: Send + Sync
{

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




// fn  build_buttondeck_neu<D: Send + Sync>(builder: &mut ButtonDeckBuilder<D>, djraw: Value, any_device: ButtonDevice /* , functions: Vec<ButtonFn>, path: P */)  -> Result<ButtonDeck<D>> {

//     let deckjson = serde_json::from_value::<DeckJson>(djraw)?;

//     let device: &dyn ButtonDeviceTrait = match &any_device {
//         ButtonDevice::Streamdeck(sd) => sd as &dyn ButtonDeviceTrait,
//         ButtonDevice::Midi(md) => md,
//     };


//     let model = device.model();

//     info!("build_buttondeck for device {}", model);

//     let opt_template = deckjson.devices
//         .and_then(|mut dv| dv.remove(&model))
//         .or_else(|| deckjson.deck);



//     let device_template = match opt_template {

//         Some(t) => t,
//         None => {

//             let json_path = builder.home_path().join(format!("{}.json", device.model()));
//             trace!("Reading config from {:?}", json_path);
        
//             let f = File::open(json_path)?;
//             serde_json::from_reader(f)?
//         }

//     };



//     Err(DeckError::NoDevice)

// }




fn  build_buttondeck<D: Send + Sync>(builder: &mut ButtonDeckBuilder<D>, mut deckjson: DeckJson, any_device: ButtonDevice /* , functions: Vec<ButtonFn>, path: P */)  -> Result<DeckDeviceSetup> {

    let deckid = 42;


    // debug!("setup::build_buttondeck {:?} with dir {:?}", &device.model(), home_folder);
    let device: &dyn ButtonDeviceTrait = match &any_device {
        ButtonDevice::Streamdeck(sd) => sd as &dyn ButtonDeviceTrait,
        ButtonDevice::Midi(md) => md,
    };

    let model = device.model();

    info!("build_buttondeck for device {}", model);

    let opt_template = deckjson.devices
        .and_then(|mut dv| dv.remove(&model))
        .or_else(|| deckjson.deck);



    let device_template = match opt_template {

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



    // --------------------------------------------------------
    // Wiring
    // --------------------------------------------------------

    let wiring = device_template.wiring;

    let maxid = wiring.iter().map(|(n,w)| w.id).max().unwrap_or(127);
    trace!("Max button id is {}", maxid);

    let mut phys: Vec<Option<PhysicalKey>> = vec![None;maxid+1];
    let mut phymap: HashMap<String,PhysicalKey> = HashMap::new();

    for (n,pt) in &wiring {
        let p: PhysicalKey = pt.into_key(n)?;
        if phys[p.id].is_some() { return  Err(DeckError::Message(format!("duplicate id: {}", p.id)));} 
        // if phymap.contains_key(&p.name) { return  Err(DeckError::Message(format!("duplicate name: {}", p.name))); }

        trace!("Physical Key: {:?}", p);
        phys[p.id] = Some(p.clone());
        phymap.insert(n.clone(), p);
    }




    // --------------------------------------------------------
    // Setups
    // --------------------------------------------------------

    // collect all setup templates
    let setups = device_template.setups
        .or_else(|| deckjson.setups)
        .unwrap_or_else(|| IndexMap::new());

    // collect all button (control) templates
    let controls = device_template.controls
        .or_else(|| deckjson.controls)
        .unwrap_or_else(|| IndexMap::new());

    // build 'prep' structs (name, reference, template) for setups
    let setup_refs: Vec<Prep<SetupRef,SetupTemplate>> = setups.iter().enumerate()
        .map(|(i,(n,t))| Prep {
            name: n,
            reference: SetupRef { id: i, name: n.clone() } ,
            template: t,
        })
        .collect();

    // build 'prep' structs (name, reference, template) for buttons
    let button_refs: Vec<Prep<ButtonId,ButtonTemplate>> = controls.iter().enumerate()
        .map(|(i,(n,t))| {
            Prep {
                name: n,
                reference: ButtonId::new(deckid, i), //  { id: i, state: None },
                template:t,
            }
        })
        .collect();


    // collect all functions (rc<refcell<>>) as name,rc tuples in a vec
    let mut functionvec: Vec<(String,Rc<RefCell<ButtonFn<D>>>)> = Vec::new();
    for (n,f) in builder.functions.drain(..) {
        functionvec.push((n, Rc::new(RefCell::new(f))))
    }

    // create references for the functions
    let function_refs: Vec<FnRef> = functionvec.iter().enumerate()
        .map(|(i,(n,f))| FnRef{ id: i, name: String::from(n) })
        .collect();


    let mut data = BuilderData {
        builder: &builder,
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
    // let (dvtx,dvrx) = std::sync::mpsc::channel::<DeviceEvent>(); 
    // let (bdtx,bdrx) = std::sync::mpsc::channel::<DeckEvent>();

    Ok(DeckDeviceSetup {
        device: Some(any_device),
        button_arena,
        current_key_map: ccm,
        wiring: phys,
        setup_arena,
        current_setup: 0,
    })

//     Ok(ButtonDeck {

//         deckid: 42,
//         hidapi: None, // will be filled later

//         device: Some(any_device),
//         // device: xdev, // Rc::new(RefCell::new(Box::new(device))),
//         folder: PathBuf::from(builder.home_path()),
//         wiring: phys,
//         functions: functionvec,
//         current_key_map: ccm,
//         button_arena,
// //         button_map,
//         current_setup: 0,
//         setup_arena,
        
//         // dummy!
//         device_event_sender: dvtx,
//         // dummy!
//         deck_event_sender: bdtx,
//         // dummy!
//         deck_event_receiver: Some(bdrx),

//         data: builder.data.take(),
//         // deckid: todo!(),

//         other: None
//     })

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
fn build_button<D: Send + Sync>(data: &BuilderData<D>, index: usize) -> Result<Button> {

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

        value: ButtonValue::from(bt.value.clone()),

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
                    value: ButtonValue::from(p.template.value.clone()),
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

