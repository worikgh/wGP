use super::NodeBox;
use super::Data;
use inputs::Inputs;
use std::cmp::Ordering;
use std;
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

#[derive(Clone, Copy)]
pub struct Score {
    // The best classification score of any single object
    pub special:f64,

    // The mean clasification score
    pub general:f64,
}

impl Score {
    pub fn evaluate(&self) -> f64 {
        // Project the two dimensional score onto one dimension for
        // sorting and selection

        // FIXME This may or may not be crucial.  Weighting of the two
        // scores may or may not be highly critical and domain
        // specific.  It will influence how the simulation leans to
        // specificity or generality
        (self.general * self.general + self.special * self.special).sqrt()
    }
    pub fn partial_cmp(&self, other:&Score) -> Option<Ordering> {
        // For ordering array of scores
        self.evaluate().partial_cmp(&other.evaluate())
    }
    pub fn copy(&self) -> Score {
        Score{general:self.general, special:self.special}
    }
    pub fn is_finite(&self) -> bool {
        self.special.is_finite() && self.general.is_finite()
    }
}

// Calculate the score of a indvidual against the data Param n: The
// individual Param d: The data to use.  'use_testing' is true if the
// individual is to be scored on the testing set.

pub fn score_individual(node:&NodeBox,
                        d:&Data,
                        use_testing:bool) -> Score {

    // Get the data to do the evaluation on
    let mut inputs = Inputs::new();
    let index:&Vec<usize>;
    if use_testing {
        index = &d.testing_i;
    }else{
        index = &d.training_i;
    }

    // To calculate the mean count examples
    let n = index.len() as f64;

    // Store each distance from the estimate to the actual value
    // to calculate best and mean estimate
    let mut y_d:Vec<f64> = Vec::new(); // Distances
    let mut s_i = std::f64::MAX; // The minimum score
    
    for i in index {
        // Examine each example
        let ref r = d.ith_row(*i);
        for j in 0..d.names.len() {
            let v:f64 = r[j];
            let h = d.names[j].clone();
            inputs.insert(h.as_str(), v);
        }

        let e = node.evaluate(&inputs).unwrap();
        // Get the target
        let t:f64 = *inputs.get(d.names.last().unwrap()).unwrap();
        let s = (t-e).abs();
        if s < s_i {
            s_i = s;
        }
        y_d.push(s);
    }

    // Calculate the two scores


    // The mean distance
    let s_m:f64 = y_d.iter().fold(0.0, |mut sum, val| {sum += val; sum})/n;
    Score{special:s_i, general:s_m}
}

