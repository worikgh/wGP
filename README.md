# Genetic Programming - Again #

## Learning Classifier Rules

A system using genetic programming to automatically generate
classifiers.  

Each classifier is a genetic programming tree expressed recursively:

```rust
 struct Node {
   o:Operator,
   l:Option<Node>,
   r:Option<Node>,
   d:Option<Node>,
 }
```

Each classifier is evolved using the genetic programming algorithm
with training data. Each classifier is associated with  a `Score` object:

```rust
#[derive(PartialEq, Debug, Clone)]
pub struct Score {
    pub quality:f64,
    pub class:String,
}
```

Score::quality is defined as the `1/(1+S)`, `S` is the sum of the
classification error over the training data.

Classification error is currently calculated using `Hinge Loss`.
Given T in 1.0, -1.0 the true classification of the case and Y the
estimate of the classifier, Hinge Loss is:

```C
1.0-T*Y < 0 ? 0.0 : 1.0-T*Y
```

A classifier is assigned to a class by evaluating a `Score` for each
class and choosing the one with the highest score.

FIXME: There is no measure of differentiation.  How good a classifier
is at telling one class from another.

When it classifies a case the ideal classifier will output 1 if the
case is of the class and -1 if the case is not.  Classifiers are
implemented as programme trees and prepared using genetic programming.

To classify a new case a collection, `Forest`, of trees is used.  Each
classifier examines the case and produces a result in [-1.0, 1.0].

For each class, calculate the mean score for classifiers in the Forest
specialised for that class, as:


```
sum(result * Score::quality)/#classifiers
```

The class with the highest score is selected.

The value returned from `classify` in population.rs is 
```
Option<(String, String)>
```

The first `String` in the 2-tuple is the winning class.  The second
string is in the format: `<class> <score>,...` listing all classes and
the score for the class in descending score order.


## Operators implemented ##

* Terminals All terminals are implemented as floating point values.
  On terminal nodes `Node::l`, `Node::r` and `Node::d` are `None`.
   * Inputs. From the domain of the function.
   * Constants. C
* Arity one functions. Apply to `Node::l`. `Node::r` and `Node::d` are None.
   * Invert. Returns one over the value of `l`.
   * Negate. Returns negative one times 'l'
* Arity two functions. Apply to `Node::l` and `Node::r`. `Node::d` is None
   * Multiply. Returns `l` times `r`.
   * Gt. If the value of `l` is greater than the value of 'r return 1.
   Otherwise return -1

   * Lt. If the value of `l` is less than the value of 'r return 1.
     Otherwise return -1

   * Addition. Returns `l` plus `r`


* Arity three functions.  Apply to `Node::l` and `Node::r`, and
     `Node::d`
   
   * If. Calculate the value or 'd'.  If that is less than or equal to
     zero return the value of `l`, otherwise the value of `r`

## Classes and Quality

## Configuration File

 Format is:

`<key> <value>`

### num_generations ###

    The number of generations to simulate 

    Example: num_generations 20000

### max_population ###

    The maximum size of the population

    Example: max_population 10000

### crossover_percent ###

    Each generation new individuals are created by combining two
    individuals.  The number of new individuals created is at most
    population x crossover_percent / 100.  New individuals are only
    added if they are not already in the population, duplicates are
    not allowed in the population.

    Example: crossover_percent 50

### training_percent ###

    The data supplied for the simulation is divided into training and
    testing.  This sets the percentage of data used to train the
    model.  During model development the training data is used.  When
    a new model that performs best in the training data is discovered
    it is run against the testing data and the results recorded

    Example: training_percent 80

### data_file ###

    The file name of the training and testing data.  Comma separated
    line per record.  The first line is the field names (these
    constitute the inputs to the generated functions).  The last
    column is the objective value.

    Example: data_file Abalone.in
    

### generations_file ###

    The name of the file out to which a line is written every
    generation

    Example: generations_file AbaloneGenerations.txt

### birthsanddeaths_file ###

    Every individual has a line in this file when it is created and
    when it is destroyed.

    Example: birthsanddeaths_file AbaloneBirthsAndDeaths.txt

### copy_prob ###







### data_file ###
### filter ###
### mutate_prob ###
### rescore ###
### save_file ###
### training_percent ###
### work_dir ###
