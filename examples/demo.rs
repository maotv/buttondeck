use std::{thread, sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};

use buttondeck::{DeckError, ButtonDeck, ButtonFn, ButtonDeckBuilder, DeviceKind, DeckEvent, FnArg, ButtonDeckSender};
use buttondeck::ButtonImage;
use hidapi::HidApi;
use log::{error, warn, info};

type Result<T> = std::result::Result<T,DeckError>;

fn simplefunc() -> Result<()>  {
    warn!("simplefunc!");
    Ok(())
}

fn customfunc<D>(d: &mut ButtonDeck<D>, e: FnArg) -> Result<()> 
    where D: Send + Sync + 'static
{
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

    let mut deck = ButtonDeckBuilder::<()>::new(DeviceKind::StreamDeck)
        .with_config("demo/deck.json")
        .with_functions(functions)
        .with_hidapi(HidApi::new()?)
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

    
//     let p: Path = Path::from("mao");
    
    loop {

        // let mut user_input = String::new();
        warn!("icecream");
        let image = ButtonImage::from_path("demo/noto/food/emoji_u1f370.png");
        sender.set_image("donut", image);
        thread::sleep(Duration::from_millis(3000));
        warn!("donut");
        let image = ButtonImage::from_path("demo/noto/food/emoji_u1f369.png");
        sender.set_image("donut", image);
        thread::sleep(Duration::from_millis(3000));



        // let stdin = io::stdin(); // We get `Stdin` here.
        // stdin.read_line(&mut user_input);
        // let clean_input = user_input.trim();
    
        // println!("input <{}>", clean_input);
        // sender.send(DeckEvent::FnCall("mute_notify".to_owned(), FnArg::Bool(true)));
        // thread::sleep(Duration::from_millis(1000))
    }

}