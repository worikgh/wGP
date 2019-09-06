#[macro_use] extern crate lazy_static;
extern crate fs2;
extern crate rand;
extern crate statistical;
mod config;
mod controller;
mod data;
mod inputs;
mod node;
mod population;
mod rng;
mod score;
use config::Config;
use data::Data;
use node::NodeBox;
use population::Population;
use score::score_individual;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;
use std::time::SystemTime;

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
/// Manage writing data to a file.  The constructor takes a file name
/// and creates a buffered writer to the file.  Has a "write" method
/// that sends data to that buffer for writing to the file.  The
/// SystemTime object is used to prefix the number of elapsed seconds
/// to each record

pub struct Recorder {
    buffer:BufWriter<File>,
    created:SystemTime,
}
impl Recorder {
    // FIXME This is really bad.  This writes to files with no way of
    // stopping two Recorders writing to the same file in confusing
    // ways

    /// new Create a recorder that writes to the file named in the argument passed. QZRT
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
        // FIXME Thread safety.
        let now = format!("{} ", self.created.elapsed().unwrap().as_secs());
        self.buffer.write(&now.into_bytes()[..]).unwrap();
        self.buffer.write(&line.to_string().into_bytes()[..]).unwrap();
        self.buffer.write(&"\n".to_string().into_bytes()[..]).unwrap();
    }
}

/// Entry point. Configuration file passed on command line as only
/// argument

fn main() {

    // Get the configuration file and build a Config object from it
    let args: Vec<String> = env::args().collect();
    eprintln!("args: {:?}", args);
    if args.len() != 2 {
        panic!("Call with one argument only.  The configuration file");
    }
    let config = Config::new(args[1].as_str());

    let mut population = Population::new(&config);
    population.start().unwrap();
    eprintln!("Simulation complete\n{}", population.report());
}
