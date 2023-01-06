use buttondeck::{DeckError, ButtonDeck, ButtonFn, ButtonDeckBuilder, DeviceKind, DeckEvent, FnArg};
use log::{error, warn, info, debug};



fn main() {
    
    env_logger::init();
    if let Err(e) = main_with_result() {
        error!("Main: {:?}", e)
    }
    
}

fn main_with_result() -> Result<(),DeckError> {

    let args: Vec<String> = std::env::args().collect();

    info!("Hello, midi!");

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::<()>::new(DeviceKind::GenericMidi)
        .with_config("demo/midi.json")
        .with_function("one", |a,b| { Ok(()) } )
        .build()?;
        // .with_functions(functions)
        // .build_first_midi()?;

    // run with current thread
    debug!("now run");
    deck.run();

    Ok(())
}
