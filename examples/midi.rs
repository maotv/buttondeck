use buttondeck::{DeckError, ButtonDeck, BtnRef, ButtonFn, ButtonDeckBuilder, DeviceKind};
use log::{error, warn, info, debug};


fn simplefunc() {
    warn!("simplefunc!");
}

fn customfunc(d: &mut ButtonDeck, b: &BtnRef) {
    warn!("Customfunc!");
}

fn main() {
    
    env_logger::init();
    if let Err(e) = main_with_result() {
        error!("Main: {:?}", e)
    }
    
}

fn main_with_result() -> Result<(),DeckError> {

    let args: Vec<String> = std::env::args().collect();

    let e = || simplefunc();

    let functions = vec![
        ButtonFn::NoArg(String::from("one"), simplefunc),
        ButtonFn::DeckArgs(String::from("two"), customfunc),
    ];

    info!("Hello, midi!");

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::new(DeviceKind::GenericMidi)
        .with_config("demo/midi.json")
        .build()?;
        // .with_functions(functions)
        // .build_first_midi()?;

    // run with current thread
    debug!("now run");
    deck.run();

    Ok(())
}
