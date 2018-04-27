# Genetic Programming - Again #

A system for automatically finding functions.

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

** Inputs. From the domain of the function.

** Constants.

* Arity one functions. Apply to `Node::l`. `Node::r` and `Node::d` are null.

** Invert. Returns one over the value of `l`.

** Negate. Returns negative one times 'l'

* Arity two functions. Apply to `Node::l` and `Node::r`. `Node::d` is null

** Multiply. Returns `l` times `r`.

** Gt. If the value of `l` is greater than the value of 'r return 1.
   Otherwise return -1

** Lt. If the value of `l` is less than the value of 'r return 1.
   Otherwise return -1

** Addition. Returns `l` plus `r`

* Arity three functions.  Apply to `Node::l` and `Node::r`, and
  `Node::d`

** If. Calculate the value or 'd'.  If that is less than or equal to
   zero return the value of `l`, otherwise the value of `r`



           