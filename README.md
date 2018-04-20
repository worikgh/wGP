Worik's take on Genetic Programming

E: A AST tree representing a GP
T: A member of the terminal set
O: A operator from the operator set

E -> T
  |  E+O

If we restrict the terminals to floats the programme will be simpler
but not as expressive as if we allow symbols.  

Members of the operator set have arity.  In the first instance allowing:

  Addition, Multiplication, Inversion and negation.  The first two
  have arity greater than one.  For simplicity it will be decleared as
  two.  inversion adn negation have arity strictly one.

