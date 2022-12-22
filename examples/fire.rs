use buttondeck::{DeckError, ButtonDeck, BtnRef, ButtonFn, ButtonDeckBuilder, DeckEvent, FnArg};
use log::{error, warn, info};




fn main() {
    
    env_logger::init();
    if let Err(e) = main_with_result() {
        error!("Main: {:?}", e)
    }
    
}

fn main_with_result() -> Result<(),DeckError> {

    let args: Vec<String> = std::env::args().collect();


    info!("Hello, world!");

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::new(buttondeck::DeviceKind::AkaiFire)
        .with_config("fire/deck.json")
        .build()?;



    // start with a new thread
    // deck.start();

    // run with current thread
    deck.run();

    Ok(())
}
