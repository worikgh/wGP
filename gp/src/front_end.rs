//use population::Population;
//use std::collections::hash_map::Entry::Occupied;
//use std::collections::hash_map::Entry::Vacant;
use std::fs;
//use std::fs::File;
use std::path::Path;
use std::env;

use cursive::Cursive;
//use cursive::event::Key;
use cursive::view::*;
use cursive::views::*;
use std::rc::Rc;
use std::sync::mpsc;

pub struct FrontEnd {
    cursive: Cursive,
    fe_rx: mpsc::Receiver<FrontEndMessage>,
    fe_tx: mpsc::Sender<FrontEndMessage>,
    controller_tx: mpsc::Sender<ControllerMessage>,
}

pub enum FrontEndMessage {
    UpdateOutput(String),
}

impl FrontEnd {
    /// Create a new FrontEnd object.  The provided `mpsc` sender will be used
    /// by the UI to send messages to the controller.
    pub fn new(controller_tx: mpsc::Sender<ControllerMessage>,
               root_dir:String,
    ) -> FrontEnd {

        // Initialise the communication channels
        let (fe_tx, fe_rx) = mpsc::channel::<FrontEndMessage>();
        

        let mut fe = FrontEnd {
            cursive: Cursive::new(),
            fe_tx: fe_tx,
            fe_rx: fe_rx,
            controller_tx: controller_tx,
        };

        
        // Create a view tree with a SelectView holding all projects

        // Collect the models
        let mut proj_dir = root_dir;
        proj_dir.push_str("/Data/");

        let proj_dir = Path::new(proj_dir.as_str());
        let entries = match fs::read_dir(proj_dir) {
            Ok(v) => v,
            Err(e) => {
                let s = format!("Failed reading {:?} e: {} cd: {:?}", proj_dir.to_str(), e, env::current_dir());
                panic!(s);
            }
        };
        let mut sv = SelectView::<String>::new().with_id("model_list");
        for e in entries {
            let p = e.unwrap().path();
            let md = p.metadata().expect("metadata call failed");
            if md.file_type().is_dir() {
                // Projects are in sub directories.
                let sp = p.file_name().unwrap().to_str().unwrap().to_string();
                //sv.get_mut().add_item_str(sp);
            }
        }

        
        // Communication channel for the "choose" button
        let controller_tx_clone = fe.controller_tx.clone();
        let choose = Button::new("Choose", move |c| {
            // When the user , send a message to the controller asking
            // to load that model
            eprintln!("In choose button 1");
            
            let sv:ViewRef<SelectView> =  match c.find_id::<SelectView>("model_list"){
                Some(sv) => sv,
                None => {
                    eprintln!("Here: 81");
                    panic!("Ouch!");
                },
            };
            eprintln!("In choose button 2");
            let model_rc = sv.selection();
            eprintln!("In choose button 2.5 model_rc: {}", model_rc);
            match   Rc::try_unwrap(model_rc) {
                Ok(model) => {
                    eprintln!("clicked  choose button: {}", model);
                
                    let message = format!("load {}", model);
                    controller_tx_clone.send(
                        ControllerMessage::UpdatedInputAvailable(message))
                        .unwrap();
                },
                Err(e) =>   eprintln!("In choose button FAILED {} ", e),
            };
            eprintln!("In choose button 3");            
        });
        eprintln!("Built choose button");
        fe.cursive.add_layer(LinearLayout::vertical()
                             .child(sv)
                             .child(choose)
        );
        
        fe
    }

    /// Step the UI by calling into Cursive's step function, then
    /// processing any UI messages.
    pub fn step(&mut self) -> bool {
        if !self.cursive.is_running() {
            return false;
        }

        // Process any pending UI messages
        while let Some(message) = self.fe_rx.try_iter().next() {
            match message {
                FrontEndMessage::UpdateOutput(text) => {
                    let mut output = self.cursive
                        .find_id::<TextView>("output")
                        .unwrap();
                    output.set_content(text);
                }
            }
        }

        // Step the UI
        self.cursive.step();

        true
    }
}

pub struct Controller {
    rx: mpsc::Receiver<ControllerMessage>,
    ui: FrontEnd,
}

pub enum ControllerMessage {
    UpdatedInputAvailable(String),
}

impl Controller {
    /// Create a new controller
    pub fn new(root_dir:String) -> Result<Controller, String> {
        let (tx, rx) = mpsc::channel::<ControllerMessage>();
        Ok(Controller {
            rx: rx,
            ui: FrontEnd::new(tx.clone(), root_dir),
        })
    }
    /// Run the controller
    pub fn run(&mut self) {
        while self.ui.step() {
            while let Some(message) = self.rx.try_iter().next() {
                // Handle messages arriving from the UI.
                match message {
                    ControllerMessage::UpdatedInputAvailable(text) => {
                        eprintln!("Received Controller::run {}", text);
                        self.ui
                            .fe_tx
                            .send(FrontEndMessage::UpdateOutput(text))
                            .unwrap();
                    }
                };
            }
        }
    }
}

pub fn go(root_dir:String) {
    // Launch the controller and UI
    let controller = Controller::new(root_dir);
    match controller {
        Ok(mut controller) => controller.run(),
        Err(e) => println!("Error: {}", e),
    };
}
