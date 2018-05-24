use rand::{ChaChaRng, Rand, Rng, SeedableRng};
use std::sync::{self, Mutex};
use rand::distributions::range::SampleRange;
lazy_static! {
    static ref RNG: Mutex<ChaChaRng> = {
        let rng = ::rand::random();
        Mutex::new(rng)
    };
}


// Convenience method
pub fn reseed(seed: &[u32]) {
    get().reseed(seed)
}

// Convenience method
pub fn random<T: Rand>() -> T {
    get().gen()
}

pub fn gen_range<T: PartialOrd+SampleRange>(low: T, high: T) -> T{
    get().gen_range(low, high)
}
pub fn get<'a>() -> sync::MutexGuard<'a, ChaChaRng>{
    RNG.lock().expect("Attempted to borrow the Rng multiple times!")
}

