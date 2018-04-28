# Genetic Programming - Again #

A system for automatically finding functions.

Given a file of tabulated data with each row describing a function
with one or more inputs and one output find a function that maps the
inputs to the output.

A function is expressed recursively as a tree.  

```rust
 struct Node {
   o:Operator,
   l:Node,
   r:Node,
   d:Node
 }
```

## Operators implemented ##

* Terminals All terminals are implemented as floating point values.
  On terminal nodes `Node::l`, `Node::r` and `Node::d` are null.
   * Inputs. From the domain of the function.
   * Constants.
* Arity one functions. Apply to `Node::l`. `Node::r` and `Node::d` are null.
   * Invert. Returns one over the value of `l`.
   * Negate. Returns negative one times 'l'
* Arity two functions. Apply to `Node::l` and `Node::r`. `Node::d` is null
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

## Configuration File

The simulation is defined using a configuration file.  The file name
is passed as the only argument `./gp <coniguration file name`.

The format is:

`<key> <value>`

### num_generations ###

    The number of generations to simulate 

    Example: `num_generations 20000`

### initial_population ###

    The number of random individuals to initialise the population
    with.

    Example: `initial_population 2000`

### max_population ###

    The maximum size the population will grow to.

    Example: `max_population 10000`

### crossover_percent ###

    Each generation new individuals are created by combining two
    individuals.  The number of new individuals created is at most
    population x crossover_percent / 100.  New individuals are only
    added if they are not already in the population, duplicates are
    not allowed in the population.

    Example: `crossover_percent 50`

### training_percent ###

    The data supplied for the simulation is divided into traing and
    testing.  This sets the percentage of data used to train the
    model.  During model development the training data is used.  When
    a new model that peforms best in the training data is discovered
    it is run against the testing data and the results recorded

    Example: `training_percent 80`

### data_file ###

    The file name of the training and testing data.  Comma seperated
    line per record.  The first line is the field names (these
    constitute the inputs to teh generated functions).  The last
    column is the objective value.

    Example: `data_file Abalone.in`
    
### model_data_file ###

    The name of the file that records the simulations of the functions
    as they are discoverd Each time a fiunction is developped that
    peforms better than any other discovered so far it is simulated
    using the testing data and the results are recorded here

    Example: `model_data_file Abalone.txt`

### r_script_file ###

    At the end of the simulaton a `R` script is written out that will
    generate raphs summerising the simulation.  This names that
    file.

    Example: `r_script_file Abalone.R`

### plot_file ###

    The name of the graphic file that the `R` script generates.

    Example: `plot_file Abalone.png`

### plot_xlab ###

### generations_file ###

    Example: `generations_file AbaloneGenerations.txt`

### birthsanddeaths_file ###

    Example: `birthsanddeaths_file AbaloneBirthsAndDeaths.txt`