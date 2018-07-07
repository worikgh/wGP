use config::Config;
use config_default::ConfigDefault;
// use cursive::Cursive;
// use cursive::views::{Dialog, TextView};
use population::Population;
use population::PopulationStatus;
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::env;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, Arc};
use std::thread;

pub struct FrontEnd {
    handles:HashMap<String, (Arc<Mutex<PopulationStatus>>, thread::JoinHandle<()>)>,
    root_dir:String,
}

impl FrontEnd {
    pub fn new () -> FrontEnd {
        FrontEnd{handles:HashMap::new(),
                 root_dir:env::current_dir().unwrap().to_str().unwrap().to_string(),
        }
    }
    pub fn fe_start(&mut self) {
        // let mut siv = Cursive::new();

        // // Creates a dialog with a single "Quit" button
        // siv.add_layer(Dialog::around(TextView::new("Hello Dialog!"))
        //               .title("Cursive")
        //               .button("Quit", |s| s.quit()));

        // // Starts the event loop.
        // siv.run();
    }
}

