// This holds a class that can be used to control many simulations.
// Simulations are run in threads and a record is kept of each thread
// ith the handle plus any other information that may be useful (Root
// directory of simulation, configuration object, time it started....)

//use population::_ensure_forest;
use population::Forest;
use config::Config;
use config_default::ConfigDefault;
use population::Population;
use population::PopulationAnalysis;
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
//use std::fs::File;
//use std::path::Path;
use std::sync::{Mutex, Arc, RwLock};
use std::thread;
use std::time::Instant;

#[derive(PartialEq, Debug, Clone)]
pub enum SimulationCommand {
    // Used to tell a running simulation to stop generating
    // generations and do some thing else

    // Default.  Carry on
    Empty, 

    // Run Population::analyse to report on the quality of the
    // simulation
    Analyse,

    // Run a simulation
    Simulate,
}

pub struct Controller {

    // The directory structure of simulations is constant belo this
    // root directory
    #[allow(dead_code)]
    root_dir:String,

    // Maps the name of a simulation to data about it including the forest
    handles:HashMap<String, (Arc<Mutex<SimulationStatus>>, Arc<RwLock<Forest>>, thread::JoinHandle<()>, Config)>,

    // The forests.
    pub forests:HashMap<String, Arc<RwLock<Forest>>>,
}

impl Controller {
    pub fn new(root_dir:String) -> Controller {
        Controller{
            root_dir:root_dir,
            handles:HashMap::new(),
            forests:HashMap::new(),
        }
    }
    pub fn get_status(& self, proj_name:&str) -> SimulationStatus {
        if let Some(ref entry)  = self.handles.get(proj_name) {
            {
                // entry is ref to a 4-tuple
                // (Arc<Mutex<SimulationStatus>>, Arc<RwLock<Forest>>,
                // thread::JoinHandle<()>, Config)
                let arc = &entry.0;
                arc.lock().unwrap().clone()
            }
        }else{
            SimulationStatus::new(false, SimulationCommand::Empty)
        }
    }
    
    pub fn get_command(&self, proj_name:&str) ->SimulationCommand {
        if let Some(ref entry) = self.handles.get(proj_name) {
            // entry is ref to a 4-tuple (Arc<Mutex<SimulationStatus>>, Arc<RwLock<Forest>>, thread::JoinHandle<()>, Config)
            let arc = &entry.0;
            arc.lock().unwrap().command.clone()
        }else{
            SimulationCommand::Empty
        }
    }
    pub fn set_command(& mut self, proj_name:&str, command:SimulationCommand)  {
        // FIXME Why does this automatically call _launch_thread if
        // proj_name not running?
        eprintln!("set_command {:?}", command);
        if !self.running(proj_name) {
            eprintln!("set_command {:?} for {}", command, proj_name);
            self._launch_thread(command, proj_name);
        }else{
            match self.handles.get(proj_name) {
                Some(ref entry) => {
                    eprintln!("seting_command {:?}", command);
                    // entry is ref to a 4-tuple (Arc<Mutex<SimulationStatus>>, Arc<RwLock<Forest>>, thread::JoinHandle<()>, Config)
                    let arc = &entry.0;
                    arc.lock().unwrap().command = command;
                },
                None => panic!("Project {} says running but no thread handle available", proj_name),
            };
        }
    }

    fn _launch_thread(& mut self, command:SimulationCommand, name:&str) {

        let config = ConfigDefault::new(name).specialise(name);        

        // Create the shared memory to monitor and control simulation
        let bb = Arc::new(Mutex::new(SimulationStatus::new(true, command)));

        // Create shared memory for the Forest if it does not exist
        let arc_forest:Arc<RwLock<Forest>> = self.forests.entry(name.to_string()).or_insert(Arc::new(RwLock::new(Forest::new()))).clone();
        
        // The population object is a frame work for the simulation
        let mut p = Population::new(&config);

        let h = p.run_in_thread(bb.clone(), arc_forest.clone());
        self.handles.insert(String::from(name), (bb, arc_forest, h, config.copy()));
        eprintln!("Set thread in motion");
    }

    pub fn running(&mut self, name:&str) -> bool {
        // Return true if there is a thread handle for this and it is
        // still running

        // FIXME?? This depends on the thread resetting `runnning` as
        // it exits in Population::run_in_thread
        match self.handles.entry(name.to_string()) {
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
        }
    }
        
    pub fn run_simulation(& mut self, name:&str) -> Result<usize, &str> {

        // Run a simulation.  The config structure has all the
        // information needed
        
        // Check not already running
        if !self.running(name) {
            self._launch_thread(SimulationCommand::Simulate,  name);
            Ok(0)
        }else{
            Err("Running already")
        }
    }
    pub fn get_config(& self, proj_name:&str) -> Option<Config> {
        if let Some(entry)  = self.handles.get(proj_name) {
            // FIXME If the simulation ever can write to Config object
            // this will need a lock
            Some(entry.3.copy())
        }else{
            None
        }
    }
}



