// This holds a class that can be used to control many simulations.
// Simulations are run in threads and a record is kept of each thread
// ith the handle plus any other information that may be useful (Root
// directory of simulation, configuration object, time it started....)

use config::Config;
use config_default::ConfigDefault;
use population::Population;
use population::PopulationAnalysis;
use std::collections::HashMap;    
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
//use std::fs::File;
//use std::path::Path;
use std::sync::{Mutex, Arc};
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
    pub fn get_status(& self, proj_name:&str) -> SimulationStatus {
        if let Some(ref entry)  = self.handles.get(proj_name) {
            {
                // entry is ref to a 3-tuple (Arc<SimulationStatus>, Handle, Config)
                let arc = &entry.0;
                arc.lock().unwrap().clone()
            }
        }else{
            SimulationStatus::new(false, SimulationCommand::Empty)
        }
    }
    
    pub fn get_command(&self, proj_name:&str) ->SimulationCommand {
        if let Some(ref entry) = self.handles.get(proj_name) {
            // entry is ref to a 3-tuple (Arc<SimulationStatus>, Handle, Config)
            let arc = &entry.0;
            arc.lock().unwrap().command.clone()
        }else{
            SimulationCommand::Empty
        }
    }
    pub fn set_command(&mut self, proj_name:&str, command:SimulationCommand)  {
        eprintln!("set_command {:?}", command);
        if !self.running(proj_name) {
            eprintln!("set_command {:?} for {}", command, proj_name);
            self._launch_thread(command, proj_name);
        }else{
            match self.handles.get(proj_name) {
                Some(ref entry) => {
                    eprintln!("seting_command {:?}", command);
                    // entry is ref to a 3-tuple (Arc<SimulationStatus>, Handle, Config)
                    let arc = &entry.0;
                    arc.lock().unwrap().command = command;
                },
                None => panic!("Project {} says running but no thread handle available", proj_name),
            };
        }
    }
    fn _launch_thread(&mut self, command:SimulationCommand, name:&str) {

        let config = ConfigDefault::new(name).specialise(name);        

        // Create the shared memory to monitor and control simulation
        let bb = Arc::new(Mutex::new(SimulationStatus::new(true, command)));

        // FIXME When a simulation is not running anymore, where is
        // the population?  In particular the Forest?
        panic!("Fix this!");
        let mut p = Population::new(&config);
        
        let h = p.run_in_thread(bb.clone());
        self.handles.insert(String::from(name), (bb, h, config.copy()));
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
        
    pub fn run_simulation(&mut self, name:&str) -> Result<usize, &str> {

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
            Some(entry.2.copy())
        }else{
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimulationStatus {

    // Front end unsets this to stop simulation gently
    pub cleared:bool,
    pub running:bool,
    pub generation:usize,
    pub start:Instant, // When started
    pub command:SimulationCommand,
    pub analysis:Option<PopulationAnalysis>,
    pub population:usize,
}

impl SimulationStatus {
    pub fn new(cleared:bool, command:SimulationCommand) -> SimulationStatus{
        SimulationStatus{
            start:Instant::now(),
            cleared:cleared,
            running:false,
            generation:0,
            population:0,
            command:command,
            analysis:None,
        }
    }
    pub fn copy(&self) -> SimulationStatus{
        SimulationStatus{
            start:self.start,
            cleared:self.cleared,
            running:self.running,
            generation:self.generation,
            population:self.population,
            command:self.command.clone(),
            analysis:self.analysis.clone(),
        }
    }
}


