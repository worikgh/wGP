// This holds a class that can be used to control many simulations.
// Simulations are run in threads and a record is kept of each thread
// ith the handle plus any other information that may be useful (Root
// directory of simulation, configuration object, time it started....)

use std::fs::File;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use config::Config;
//use std::path::Path;
use population::Population;
use std::collections::HashMap;    
use std::sync::{Mutex, Arc};
use std::thread;
pub struct Controller {
    root_dir:String,
    handles:HashMap<String, (Arc<Mutex<SimulationStatus>>, thread::JoinHandle<()>)>,
}

impl Controller {
    pub fn new(root_dir:String) -> Controller {
        Controller{
            root_dir:root_dir,
            handles:HashMap::new(),
        }
    }
    pub fn get_status(& mut self, proj_name:&String) -> SimulationStatus {
        if let Occupied(entry)  = self.handles.entry(proj_name.to_string()) {
            {
                (*entry.get().0.lock().unwrap()).copy()
            }
        }else{
            SimulationStatus::new()
        }
    }
    pub fn run_simulation(&mut self, mut config: Config) -> Result<usize, &str> {

        // Run a simulation.  The config structure has all the
        // information needed

        // Get the project configuration and over write the defaults.
        // FIXME Should this be done here or in
        // Controller::run_simulation?
        let proj_dir = format!("{}Data/", config.get_string("root_dir").unwrap());
        let cfg_fname = format!("{}{}/.gp_config", proj_dir, config.get_string("name").unwrap());

        if let Ok(f) = File::open(cfg_fname.clone()) {
            let _cfg = Config::new_file(f);
            for key in _cfg.data.keys() {
                // Over ride a default
                let g = _cfg.get_string(&key).unwrap();
                let v = g.clone();
                eprintln!("Controller::run_simulation: key {} v {}", key, v);
                config.data.insert(key.clone(), v);
                eprintln!("Controller::run_simulation: Done");
            }
        }else{
            eprintln!("Cannot find local config: {}", cfg_fname);
        };
        // Ensure simulation knows where to run
        config.data.insert("proj_dir".to_string(), proj_dir);
        
        // Check not already running
        let name = config.get_string("name").unwrap();
        let running = match self.handles.entry(name.to_string()) {
            Occupied(o) => {
                let o1 = o.get();
                let ps = &*(o1.0).lock().unwrap();
                if ps.running {
                    true
                }else{
                    false
                }
            },
            Vacant(_) => {
                eprintln!("Not Running");
                false
            }
        };
        if !running {

            // Create the shared memory to monitor and control simulation
            let bb = Arc::new(Mutex::new(SimulationStatus{running:false,
                                                          cleared:true,
                                                          generation:0,
                                                          path:config.get_string("proj_dir").unwrap(),
            }));
            let h = Population::new_sub_thread(config, bb.clone());
            self.handles.insert(String::from(name.as_str()), (bb, h));
            Ok(0)
        }else{
            Err("Running already")
        }
    }
}

impl SimulationStatus {
    pub fn new() -> SimulationStatus{
        SimulationStatus{
            cleared:false,
            running:false,
            generation:0,
            path:"".to_string(),
        }
    }
    pub fn copy(&self) -> SimulationStatus{
        SimulationStatus{
            cleared:self.cleared,
            running:self.running,
            generation:self.generation,
            path:self.path.clone(),
        }
    }
}

pub struct SimulationStatus {

    // Front end unsets this to stop simulation gently
    pub cleared:bool,
    pub running:bool,
    pub generation:usize,
    pub path:String, // FIXME This should be a reference
}

