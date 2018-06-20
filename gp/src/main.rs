#[macro_use] extern crate lazy_static;
extern crate rand;
extern crate statistical;

mod config;
mod data;
mod inputs;
mod node;
mod population;
mod rng;
mod score;
use config::Config;
use data::Data;
use population::Population;
use score::score_individual;
use std::env;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;

use std::time::SystemTime;

// The type of data that can be a terminal
#[derive(Debug)]
enum TerminalType {
    Float(f64),
    // Custom terminals for inputs
    Inputf64(String),
}

// Get the data from the terminal
fn gt(tt:&TerminalType) -> String {
    match tt {
        &TerminalType::Float(f) => format!("Float {} ",f),
        &TerminalType::Inputf64(ref s) => format!("{} ",s),
    }
}

impl fmt::Display for TerminalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = gt(self);
        write!(f, "{}", n)
    }
}

use node::NodeBox;
//use node::Node;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_data_partition() {
        let config = Config::new("config");
        let data_file = config.get_string("data_file").unwrap();

        // Load the data
        let mut d_all:Data = Data::new();
        d_all.read_data(data_file.as_str(), 0);
        assert_eq!(d_all.training_i.len(), 0);

        d_all.read_data(data_file.as_str(), 100);
        assert_eq!(d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 50);
        assert_ne!(d_all.testing_i.len(), 0);
        assert_ne!(d_all.training_i.len(), 0);

        d_all.read_data(data_file.as_str(), 10);
        assert!(d_all.training_i.len()< d_all.testing_i.len(), 0);

        d_all.read_data(data_file.as_str(), 90);
        assert!(d_all.training_i.len() > d_all.testing_i.len(), 0);

    }
    #[test]
    fn test_node_eval(){
        let mut inputs = Inputs::new();
        let s = "Multiply Float 0.06 Negate Diameter";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, -0.60);

        let mut inputs = Inputs::new();
        let s = "Invert Float 0.1";
        inputs.insert("Diameter", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 10.0);

        let inputs = Inputs::new();
        let s = "Add Float 2000.0 Invert Float 0.1";
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 2010.0);

        let mut inputs = Inputs::new();
        let s = "Multiply Height Add Float 10.0 Invert Float 0.1";
        inputs.insert("Height", 10.0);
        let t = Node::new_from_string(s).evaluate(&inputs).unwrap();
        assert_eq!(t, 200.0);

        let s = "If Lt x Float 0.0 Float -1.0 Float 1.0";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", 0.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);

        let mut inputs = Inputs::new();
        inputs.insert("x", -0.01);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Gt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -1.0);        

        let s = "Lt Log If x x x x";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.0);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", 1.2);
        inputs.insert("y", 1.1);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, 1.1);        

        let s = "If Lt x y x y";
        let n = Node::new_from_string(s);
        let mut inputs = Inputs::new();
        inputs.insert("x", -9.0);
        inputs.insert("y", 1.0);
        let t = n.evaluate(&inputs).unwrap();
        assert_eq!(t, -9.0);        

    }
    #[test]
    fn test_node_from_string(){
        let s = "Add Add Add Invert Height Diameter Add Negate Float 0.03049337449511591 Add Multiply Negate Invert Float 0.40090461861005733 Negate Diameter Negate Float 0.06321754406175395 Length";
        let n = Node::new_from_string(s);
        let ns = &n.to_string()[..];
        println!("");
        println!("1: {}", s);
        println!("2: {}", ns.trim());
        assert_eq!(ns.trim(), s.to_string());        
    }
}

pub struct Recorder {
    // Manage writing data to a file.  The constructor takes a file
    // name and creates a buffered writer to the file.  Ha a "write"
    // withod that sends data to that buffer for writing to the file.
    // The SystemTime object is used to prefix the number of elapsed
    // seconds to each record
    buffer:BufWriter<File>,
    created:SystemTime,
}
impl Recorder {
    fn new(file_name:&str) -> Recorder {
        Recorder{
            buffer:BufWriter::new(OpenOptions::new()
                                  .append(true)
                                  .create(true)
                                  .open(file_name).unwrap()),
            created:SystemTime::now(),
        }
    }
    fn write_line(&mut self, line:&str) {
        let now = format!("{} ", self.created.elapsed().unwrap().as_secs());
        self.buffer.write(&now.into_bytes()[..]).unwrap();
        self.buffer.write(&line.to_string().into_bytes()[..]).unwrap();
        self.buffer.write(&"\n".to_string().into_bytes()[..]).unwrap();
    }
}

fn main() {
    println!("Start");

    // Get configuration file.  Will use file names 'config' if no
    // file name provided as a argument FIXME:  Don't have a default!!!
    let args: Vec<_> = env::args().collect();
    let cfg_file:String;
    if args.len() > 1 {
        cfg_file = args[1].clone();
    }else{
        cfg_file = "config".to_string();
    }
    let config = Config::new(cfg_file.as_str());

    // Load configuration data
    let generations_file = config.get_string("generations_file").unwrap();
    let birthsanddeaths_file = config.get_string("birthsanddeaths_file").unwrap();
    let num_generations = config.get_usize("num_generations").unwrap();
    let seed = config.get_string("seed").unwrap(); // The seed is a string of usize numbers
    let seed:Vec<u32> = seed.split_whitespace().map(|x| x.parse::<u32>().unwrap()).collect();
    let data_file = config.get_string("data_file").unwrap() ;
    let training_percent = config.get_usize("training_percent").unwrap();
    
    // Set up output file to record each generation:  FIXME move this to population
    let mut generation_recorder = Recorder::new(generations_file.as_str());


    // The source of entropy.  
    rng::reseed(seed.as_slice());

    // Create a population. 
    println!("Population start");
    let data = Data::new(&data_file, training_percent);
    let bnd_recorder = Recorder::new(birthsanddeaths_file.as_str());
    let mut population = Population::new(&config, &data, bnd_recorder);
    population.initialise();
    println!("Initial population {}", population.len());

    // Write the header for the generaion file
    let s = format!("generation, best_id, Best Score General, Best Score Special, Population, Best");
    generation_recorder.write_line(&s[..]);
    generation_recorder.buffer.flush().unwrap();

    if population.do_train() {



        for generation in 0..num_generations {
            // Main loop
            
            population.new_generation(generation);
            let s = format!("{} {} {} {} {}",
                            generation,
                            population.best_id(),
                            population.best_score().special,
                            population.len(),
                            population.get_tree_id(population.best_id()).1.to_string());
            generation_recorder.write_line(&s[..]);
            generation_recorder.buffer.flush().unwrap();
        }
    }

    if population.do_classify() {
        // Do classification
        population.classify_test();
    }
    println!("Bye!");
        
}

