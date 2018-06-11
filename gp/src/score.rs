use super::NodeBox;
use super::Data;
//use data::Class;
use inputs::Inputs;
use std::cmp::Ordering;
//use std;
// Scoring a individual is key to evolving a good population of
// individuals.

// The object of a simulation, in the general non-linear
// classification case, is to evolve a good population of
// classifiers.  Some will be generalists and be able to classify
// to more than one class, others specialists good at identifying
// a single class and others specialists over individual examples
// able to properly classify them but not others.

// Much of the evolution process is driven by scoring.  When a
// individual is selected for crossover or for reproduction it is
// selected using probabilities weighted by probability.  So here
// in is a method to do roulette wheel selection.

// Ordering the population is more difficult with a structured
// score.  It is not possible to intuitively sort a one
// dimensional array with a 2-dimensional score.

// Individual's score can be used to classify the individuals of a
// population.  Given a function domain that maps objects to
// classifications individuals that classify the same set of
// objects to the correct classifiers are related in a functional
// sense.  This seems like a fruitful area for research.

// To this end the score of a individual is a struct not a f64

#[derive(Debug, Clone)]
pub struct Score {
    // Fitness calculated hen classifying to self.class.unwrap()
    pub special:f64,

    //  The name of the class this score is specialised for. Obtained
    //  from the objective data
    pub class:String,
}

impl Score {
    // pub fn new(id:usize) -> Score {
    //     Score {id:id, special:0.0, general:0.0, initialise:false}
    // }

    // pub fn Copy(&self) -> Score {
    //     Score{class:self.class.clone(), special:self.special}
    // }
    
    pub fn evaluate(&self) -> f64 {
        self.special
    }
    
    pub fn partial_cmp(&self, other:&Score) -> Option<Ordering> {
        // For ordering array of scores
        self.evaluate().partial_cmp(&other.evaluate())
    }
    pub fn is_finite(&self) -> bool {
        self.special.is_finite()
    }

    pub fn copy(&self) -> Score {
        let class = self.class.clone();
        Score{special:self.special, class:class}
    }
    // Calculate the score of a indvidual against the data Param n: The
    // individual Param d: The data to use.  'use_testing' is true if the
    // individual is to be scored on the testing set.

}


pub fn score_individual(
    node:&NodeBox,
    d:&Data,
    use_testing:bool) -> Score {

    // Score individual is called once per node on creation. FIXME
    // Call it from Node::new

    // Get the data to do the evaluation on
    let mut inputs = Inputs::new();
    let index:&Vec<usize>;
    if use_testing {
        index = &d.testing_i;
    }else{
        index = &d.training_i;
    }

    let mut c:Option<String> = None;
    let mut best_s = 0.0;

    for  class in d.class_names.iter() {
        // Store each distance from the estimate to the actual value
        // to calculate best and mean estimate
        let mut y_d:Vec<f64> = Vec::new(); // Distances

        let n = d.class_names.len();
        
        for i in index {

            // Examine each example

            // The data for this example
            let ref r = d.ith_row(*i);

            // Prepare the inputs to the simulation
            for j in 0..d.input_names.len() {
                let v:f64 = r[j];
                let h = d.input_names[j].clone();
                inputs.insert(h.as_str(), v);
            }

            // Is this example in the class of this node?
            let _c = d.ith_row(*i);
            let t = _c[d.class_idx(class)];

            // Get the estimate
            let e = node.evaluate(&inputs).unwrap();

            // Score: See ExperimentalResults.tex for explanation of how
            // this is calculated
            if t == 1.0 {
                // This individual is classifying for the class of this
                // example.  
                y_d.push(1.0/(1.0+(1.0 - e).abs()))
            }else{
                y_d.push(-1.0/((n as f64)*(1.0+(1.0 - e).abs())))
            }
        }
        let mut s = (1.0/(index.len() as f64))*y_d.iter().fold(0.0, |mut sum, &x| {sum += x; sum});
        if !s.is_finite() || s < 0.0 { s = 0.0 }

        if s >= best_s {
            c = Some(class.clone());
            best_s = s;
        }
    }

    Score{special:best_s,
          // FIXME This should be a reference with a life time.  This
          // string should be in population.class_names only
          class:c.unwrap()
    }
}

