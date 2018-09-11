
use std::path::Path;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
//use std::env;


pub struct Config {
    pub data:HashMap<String, String>,
}

impl Config {
    pub fn copy(&self) -> Config {
        let mut data:HashMap<String, String> = HashMap::new();
        for k in self.data.keys() {
            data.insert(k.clone(), self.data.get(k).unwrap().clone());
        }
        Config{data:data}
    }
    #[allow(dead_code)]
    pub fn new(cfg_file:&str)-> Config {
        let file = File::open(cfg_file).unwrap();
        Config::new_file(file)
    }
    pub fn new_file(file:File) -> Config {
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let lines = contents.lines();
        let mut config_hm = HashMap::new();

        for line in lines {
            let mut iter = line.split_whitespace();

            // This will ignore blank lines
            if let Some(k)  = iter.next() {
                let v = iter.map(|x| format!("{} ", x)).collect::<String>();
                if k != "#" {
                    config_hm.insert(k.to_string(), v.trim().to_string());
                }
            }
        }
        Config{data:config_hm}
    }
    fn _get(&self, k:&str) -> &str {
        match self.data.get(k) {
            Some(v) => v,
            _ => panic!("Failed config. {} as usize", k),
        }
    }
    #[allow(dead_code)]
    pub fn specialise(&self, name:&str) -> Config {

        // Specialise a configuration object for running a particular
        // project named in @param name

        // The configuration must have 'root_dir' set.  This should be
        // tha same for all projects.  If a .gp_config file cannot be
        // found return a copy of passed in Config object
        
        let mut ret = self.copy();
        ret.data.insert("name".to_string(), name.to_string());
        let proj_dir = format!("{}/Data/", self.get_string("root_dir").unwrap());
        let cfg_fname = format!("{}{}/.gp_config", proj_dir,  name);

        let path = Path::new(&cfg_fname);
        if let Ok(f) = File::open(path) {
            let _cfg = Config::new_file(f);
            for (k, v) in _cfg.data.iter() {
                // Over ride a default
                ret.data.insert(k.to_string(), v.to_string());
            }
        }else{
            eprintln!("Cannot find local config file: {}", cfg_fname);
        };

        ret
    }
    #[allow(dead_code)]
    pub fn get_f64(&self, k:&str) -> Option<f64> {
        match self._get(k).parse::<f64>() {
            Ok(v) => Some(v),
            _ => None,
        }
    }
    pub fn get_usize(&self, k:&str) -> Option<usize> {
        match self._get(k).parse::<usize>() {
            Ok(x) => Some(x),
            _ => None,
        }
    }
    pub fn get_string(&self, k:&str) -> Option<String> {
        match self.data.get(k) {
            Some(v) => Some(v.clone()),
            _ => None,
        }
    }        
}
