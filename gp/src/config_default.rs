// When all else fails this is a configuration that will actually run
use std::collections::HashMap;
use config::Config;
use std::env;

pub struct ConfigDefault {
}

impl ConfigDefault {
    pub fn new(name:&str) -> Config {
        // @param `name` is the name of the project

        let mut data:HashMap<String, String> = HashMap::new();

        // Using a fixed seed makes runs deterministic and debugging
        // much simpler
        data.insert("seed".to_string(), "11 2 3 120".to_string());

        // A small population and just enough generations to test
        data.insert("max_population".to_string(), "100".to_string());
        data.insert("num_generations".to_string(), "2".to_string());

        // The working directory defaults to the <starting
        // directory>/Data/<name of simulation>
        data.insert("root_dir".to_string(), env::current_dir().unwrap().to_str().unwrap().to_string());

        // The default location under root for the directory holding
        // project files
        let mut work_dir = data.get("root_dir").unwrap().clone();
        work_dir.push_str("/Data/");
        work_dir.push_str(name);
        work_dir.push('/');
        data.insert("work_dir".to_string(), work_dir.clone());

        // Probability that a individual will be copied
        data.insert("copy_prob".to_string(), "50".to_string());

        // The proportion of the population that uses crossover each
        // generation
        data.insert("crossover_percent".to_string(), "50".to_string());

        // The percentage of data to hold back for testing
        data.insert("training_percent".to_string(), "10".to_string());

        data.insert("birthsanddeaths_file".to_string(), format!("{}{}_BnD.txt", work_dir, name).to_string());
        data.insert("data_file".to_string(), "data.in".to_string());
        data.insert("filter".to_string(), "1".to_string());
        data.insert("generations_file".to_string(), format!("{}{}_Generations.txt", work_dir, name).to_string());
        data.insert("mutate_prob".to_string(), "1".to_string());
        data.insert("rescore".to_string(), "0".to_string());
        data.insert("save_file".to_string(), format!("{}{}_Trees.txt", work_dir, name).to_string());
        
        Config{data:data}
    }
}
