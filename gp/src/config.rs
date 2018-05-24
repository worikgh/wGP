
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

pub struct Config {
    data:HashMap<String, String>,
}

impl Config {
    pub fn new(cfg_file:&str)-> Config {
        let file = File::open(cfg_file).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let lines = contents.lines();
        let mut config_hm = HashMap::new();
        for line in lines {
            let mut iter = line.split_whitespace();
            let k = iter.next().unwrap();
            let v = iter.map(|x| format!("{} ", x)).collect::<String>();
            if k != "#" {
                config_hm.insert(k.to_string(), v.trim().to_string());
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
    pub fn get_f64(&self, k:&str) -> Option<f64> {
        match self._get(k).parse::<f64>() {
            Ok(v) => Some(v),
            _ => None,
        }
    }
    pub fn get_usize(&self, k:&str) -> Option<usize> {
        match self._get(k).parse::<usize>() {
            Ok(v) => Some(v),
            _ => {
                None
            },
            
        }
    }
    pub fn get_string(&self, k:&str) -> Option<String> {
        match self.data.get(k) {
            Some(v) => Some(v.clone()),
            _ => None,
        }
    }        
}
