use buttondeck::{DeckError, ButtonDeck, BtnRef, ButtonFn, ButtonDeckBuilder, DeviceKind, DeckEvent, FnArg};
use log::{error, warn, info};

type Result<T> = std::result::Result<T,DeckError>;

fn simplefunc() -> Result<()>  {
    warn!("simplefunc!");
    Ok(())
}

fn customfunc(d: &mut ButtonDeck, e: FnArg) -> Result<()> {
    warn!("Customfunc!");
    d.switch_to("volume");
    Ok(())
}

fn main() {
    
    env_logger::init();
    if let Err(e) = main_with_result() {
        error!("Main: {:?}", e)
    }
    
}

fn main_with_result() -> Result<()> {

    let args: Vec<String> = std::env::args().collect();

    let e = || simplefunc();


    let bf = Box::new(simplefunc);

    let functions = vec![
        (String::from("one"), ButtonFn { func: Box::new(|a,b| simplefunc()) }),
        (String::from("two"), ButtonFn { func: Box::new(customfunc) }),
    ];

    info!("Hello, world!");

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::new(DeviceKind::StreamDeck)
        .with_config("demo/deck.json")
        .with_functions(functions)
        .build()?;



    // start with a new thread
    // deck.dump();


    // let sender = deck.get_sender();

    // run with current thread
    deck.run();

    Ok(())
}
