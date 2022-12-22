use buttondeck::{DeckError, ButtonDeck, ButtonFn, ButtonDeckBuilder, DeviceKind, DeckEvent, ButtonDeckSender, FnArg};
use log::{error, warn, info};
use std::{thread, time::Duration};

type Result<T> = std::result::Result<T,DeckError>;


fn customfunc(d: &mut ButtonDeck, e: FnArg) -> Result<()> {
    warn!("Customfunc! {}", e);
    if e.as_bool() {
        // d.set_button_state("mute", "on")
    } else {
        // d.set_button_state("mute", "off")
    }
    
    Ok(())
}

fn toggle_mute(d: &mut ButtonDeck, arg: FnArg) -> Result<()> {
    info!("Mute Button {}", arg);
    match arg {
        FnArg::Button(rb) => {
            d.toggle_button_state(&rb)?
        }
        FnArg::Bool(b) => {
            
        },
        _ => {

        }
    }

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

    info!("Hello, world!");

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::new(DeviceKind::StreamDeck)
        .with_config("demo/panoo.json")
        .with_function("mute", customfunc )
        .with_function("toggle_mute", toggle_mute )
        .build()?;



    // start with a new thread
    // deck.dump();
    let bsender = deck.get_sender();

    thread::spawn(move || {
        random_message_thread(bsender);
    });

    // run with current thread
    deck.run();

    Ok(())
}




fn random_message_thread(sender: ButtonDeckSender) {

    let mut mute = false;

    loop {
        sender.send(DeckEvent::FnCall("mute".to_owned(), FnArg::Bool(mute)));
        mute = !mute;
        thread::sleep(Duration::from_millis(10000));
    }

}