use buttondeck::{DeckError, ButtonDeck, ButtonFn, ButtonDeckBuilder, DeviceKind, DeckEvent, ButtonDeckSender, FnArg, StateRef2, ButtonId};
use log::{error, warn, info};
use std::{thread, time::Duration, io, sync::{atomic::{AtomicIsize, AtomicUsize, Ordering}, Arc}, path::Path};

type Result<T> = std::result::Result<T,DeckError>;



fn mute_notify<D>(d: &mut ButtonDeck<D>, e: FnArg) -> Result<()> {
    
    warn!("Customfunc! {}", e);
    let bid = d.button_id_from_name("mute")?;

    if e.as_bool() {
        d.set_button_state_with_id(bid, &StateRef2::from("on"))
    } else {
        d.set_button_state_with_id(bid, &StateRef2::from("off"))
    }
    
    Ok(())
}

fn toggle_mute<D>(d: &mut ButtonDeck<D>, arg: FnArg) -> Result<()> {
    info!("Mute Button {}", arg);
    match arg {
        FnArg::Button(rb, v) => {
            d.toggle_button_state(rb)?
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

    let mute_state  = Arc::new(AtomicUsize::new(0));
    let clone_state = Arc::clone(&mute_state);

    // let mut api = ButtonApi { hidapi: HidApi::new()? };
    // let mut deck = ButtonDeck::open_deck(&mut api, "demo")?;

    let mut deck = ButtonDeckBuilder::<()>::new(DeviceKind::StreamDeck)
        .with_config("demo/panoo.json")
        .with_function("mute_notify", mute_notify )
        .with_function("toggle_mute", toggle_mute )
        .build()?;



    // start with a new thread
    // deck.dump();
    let bsender = deck.get_sender();

    thread::spawn(move || {
        random_message_thread(bsender, clone_state);
    });

    // run with current thread
    deck.run();

    Ok(())
}





fn random_message_thread(sender: ButtonDeckSender, mute_state: Arc<AtomicUsize>) {

    let mut mute = false;

    
//     let p: Path = Path::from("mao");
    
    loop {

        let mut user_input = String::new();

        let stdin = io::stdin(); // We get `Stdin` here.
        stdin.read_line(&mut user_input);
        let clean_input = user_input.trim();
    
        println!("input <{}>", clean_input);
     
        match clean_input {
            "m" => {
                if mute_state.load(Ordering::SeqCst) == 0 {
                    mute_state.store(1, Ordering::SeqCst);
                    sender.send(DeckEvent::FnCall("mute_notify".to_owned(), FnArg::Bool(true)));
                } else {
                    mute_state.store(0, Ordering::SeqCst);
                    sender.send(DeckEvent::FnCall("mute_notify".to_owned(), FnArg::Bool(false)));
                }
                println!("mute <{:?}>", mute_state);
            },
            _ => {
                println!("input <{}>", clean_input);
            }
        }

    }

}