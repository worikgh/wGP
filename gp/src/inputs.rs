use std::collections::HashMap;
pub struct Inputs {
    dataf:HashMap<String, f64>,
}
impl Inputs {
    pub  fn new() -> Inputs {
        Inputs{
            dataf:HashMap::new(),
        }
    }
    pub fn  insert(&mut self, k:&str, v:f64) {
        self.dataf.insert(k.to_string(), v);
    }
    pub fn get(&self, k:&str) -> Option<&f64> {
        self.dataf.get(k)
    }
}
