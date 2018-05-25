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


// #![feature(thread_local)]

// extern crate rand;
// extern crate xorshift;

// use rand::distributions::normal::StandardNormal;
// use xorshift::{Rand,Rng,SeedableRng,SplitMix64,Xoroshiro128};

// #[thread_local]
// static mut RNG:Option<Xoroshiro128> = None;

// pub fn seed(x:u64) {
//     let mut seeding_rng:SplitMix64 = SeedableRng::from_seed(x);
//     unsafe { RNG = Some(Rand::rand(&mut seeding_rng)); }
// }

// pub fn rnorm() -> f64 {
//     unsafe {
//         let StandardNormal(x) = RNG.as_mut().unwrap().gen::<StandardNormal>();
//         x
//     }
// }
