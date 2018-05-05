/// The source of randomness for the simulations.  Constrained to one
/// structure so it can be seeded deterministically.  FIXME Why cannot I
/// just pass around a rand::StdRng?

use rand::Rng;
use rand::StdRng;
use rand::SeedableRng;

pub struct Randomness
{
    rng:StdRng,
}
impl Randomness{
    pub fn new(seed:&[usize]) -> Randomness {
        Randomness{rng:SeedableRng::from_seed(seed)}
    }
    pub fn gen(& mut self) -> f64 {
        self.rng.gen()
    }
    pub fn gen_range(& mut self, a:usize, b:usize)->usize{
        self.rng.gen_range(a,b)
    }
    pub fn gen_rangef64(& mut self, a:f64, b:f64)->f64{
        self.rng.gen_range(a,b)
    }
}


