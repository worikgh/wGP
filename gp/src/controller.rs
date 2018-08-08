// This holds a class that can be used to control many simulations.
// Simulations are run in threads and a record is kept of each thread
// ith the handle plus any other information that may be useful (Root
// directory of simulation, configuration object, time it started....)

use config::Config;
use config_default::ConfigDefault;
use population::Population;
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::fs::File;
use std::path::Path;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time::Instant;

#[derive(Clone)]
pub enum SimulationCommand {
    // Used to tell a running simulation to stop generating
    // generations and do some thing else

    // Default.  Carry on
    Empty, 

    // Run Population::analyse to report on the quality of the
    // simulation
    Analyse, 
}

pub struct Controller {

    // The directory structure of simulations is constant belo this
    // root directory
    #[allow(dead_code)]
    root_dir:String,

    // Maps the name of a simulation to data about it
    handles:HashMap<String, (Arc<Mutex<SimulationStatus>>, thread::JoinHandle<()>, Config)>,
}

impl Controller {
    pub fn new(root_dir:String) -> Controller {
        Controller{
            root_dir:root_dir,
            handles:HashMap::new(),
        }
    }
    pub fn get_status(&mut self, proj_name:&str) -> SimulationStatusReport {
        if let Occupied(entry)  = self.handles.entry(proj_name.to_string()) {
            SimulationStatusReport::new(&(*entry.get().0.lock().unwrap()).copy(), &entry.get().2)
        }else{
            SimulationStatusReport::new(&SimulationStatus::new(), &ConfigDefault::new(proj_name))
        }
    }
    
    pub fn run_simulation(&mut self, config: & Config) -> Result<usize, &str> {

        // Run a simulation.  The config structure has all the
        // information needed

        
        // Get the project configuration and over write the defaults.
        // FIXME Should this be done here or in
        // Controller::run_simulation?
        let proj_dir = format!("{}/Data/", config.get_string("root_dir").unwrap());
        let cfg_fname = format!("{}{}/.gp_config", proj_dir, config.get_string("name").unwrap());

        let mut config = config.copy();
        let path = Path::new(&cfg_fname);
        if let Ok(f) = File::open(path) {
            let _cfg = Config::new_file(f);
            for key in _cfg.data.keys() {
                // Over ride a default
                let g = _cfg.get_string(&key).unwrap();
                let v = g.clone();
                config.data.insert(key.clone(), v);
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
                                                          start:Instant::now(),
                                                          generation:0,
                                                          command:SimulationCommand::Empty,
                                                          path:config.get_string("proj_dir").unwrap(),
            }));
            let mut p = Population::new(&config);
            let h = p.run_in_thread(bb.clone());
            self.handles.insert(String::from(name.as_str()), (bb, h, config.copy()));
            Ok(0)
        }else{
            Err("Running already")
        }
    }
    pub fn get_config(&mut self, proj_name:&str) -> Option<Config> {
        if let Occupied(entry)  = self.handles.entry(proj_name.to_string()) {
            // FIXME If the simulation ever can write to Config object
            // this will need a lock
            Some(entry.get().2.copy())
        }else{
            None
        }
    }
}

impl SimulationStatus {
    pub fn new() -> SimulationStatus{
        SimulationStatus{
            start:Instant::now(),
            cleared:false,
            running:false,
            generation:0,
            path:"".to_string(),
            command:SimulationCommand::Empty,
        }
    }
    pub fn copy(&self) -> SimulationStatus{
        SimulationStatus{
            start:self.start,
            cleared:self.cleared,
            running:self.running,
            generation:self.generation,
            path:self.path.clone(),
            command:self.command.clone(),
        }
    }
}

pub struct SimulationStatus {

    // Front end unsets this to stop simulation gently
    pub cleared:bool,
    pub running:bool,
    pub generation:usize,
    pub path:String, // FIXME This should be a reference
    pub start:Instant, // When started
    pub command:SimulationCommand,
}

impl SimulationStatusReport {
    pub fn new(status:&SimulationStatus, config:&Config) -> SimulationStatusReport {
        SimulationStatusReport {
            cleared:status.cleared,
            running:status.running,
            generation:status.generation,
            path:status.path.clone(),
            started:status.start,
            max_population:config.get_usize("max_population").unwrap(),
            num_generations:config.get_usize("num_generations").unwrap(),
            created:Instant::now(),
        }
    }
    
}

pub struct SimulationStatusReport {
    pub cleared:bool,
    pub running:bool,
    pub generation:usize,
    pub path:String, // FIXME This should be a reference
    pub created:Instant, // When this report created
    pub started:Instant, // When simulation started
    pub max_population:usize,
    pub num_generations:usize,
}
