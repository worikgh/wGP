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

#[inline]
fn loss_function(y_:f64, y:f64) -> f64 {
    // y_ is the estimate, y the true value

    // True value is -1.0 if this the input is not a member of the class
    // the individual tests for, and 1.0 if it is
    
    // Hinge loss
    let loss = 1.0-y_*y;
    if loss < 0.0 {
        0.0
    }else{
        loss
    }
}

pub fn score_individual(
    node:&NodeBox,
    d:&Data,
    use_testing:bool) -> Score {

    // Score individual is called once per node.  Classifies it
    // (decides what class it is for) and gives it a rating

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

            // Is this example in the class of this node?  True value
            // is -1.0 if this the input is not a member of the class
            // the individual tests for, and 1.0 if it is for.

            let t = if r[d.class_idx(class)] > 0.0 {
                1.0
            }else{
                -1.0
            };

            // Get the estimate
            let e = node.evaluate(&inputs).unwrap();

            // Score: See ExperimentalResults.tex for explanation of
            // the loss function
            y_d.push(loss_function(t, e));
        }
        let mut s = (1.0/(index.len() as f64))*y_d.iter().fold(0.0, |mut sum, &x| {sum += x; sum});
        if !s.is_finite() || s < 0.0 { s = 0.0 }

        // The score must be increasing with quality.  The above is a
        // quantity to minimise.  Cannot use a direct inverse as a
        // zero will cause a exception.  Zero is the floor of s
        s = 1.0/(1.0+s);
        
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

