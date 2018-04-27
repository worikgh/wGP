# Genetic Programming - Again #

A system for automatically writing computer programmes.

A programme is expressed recursively as a tree.  

```rust
 struct Node {
   o:Operator,
   l:Node,
   r:Node,
   d:Node
 }
```

Operators implemented:

* Terminals

  All terminals are implemented as floating point values

** Inputs 

** Constants




The terminals are restricted to floats for  simplicity.

Operators are Addition, Multiplication, Inversion and Negation. 

Programmes are expressed as trees

Operators can be terminals (inputs or constants), arity one e.g.,
invert, arity to e.g., multiplication or arity three e.g., division
(condition, true branch, false branch).  Node::l, Node::r, Node::d are
the zero to three subtrees depending on the operator.


           