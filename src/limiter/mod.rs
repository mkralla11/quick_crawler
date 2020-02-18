use dashmap::DashMap;
use futures::prelude::*;
use futures::future::{self, join_all};
use std::time::{Duration, Instant};

use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
use ratelimit_futures::Ratelimit;
use std::num::NonZeroU32;
use futures::{
  compat::{Future01CompatExt, Stream01CompatExt},
  future::{FutureExt, TryFutureExt},
};


#[derive(Debug, Clone)]
pub struct Limiter {
    hash: DashMap<String, DirectRateLimiter>,
}


impl Limiter {
    pub fn new() -> Self {
        let hash = DashMap::new();
        // let lim = DirectRateLimiter::<LeakyBucket>::per_second(NonZeroU32::new(1).unwrap());
        Self {
            hash
        }
    }


    pub async fn limit<S: Into<String>>(&self, id: S) -> Result<(), String> {
        let id = id.into();
        let lim = match self.hash.get(&id) {
            Some(limiter)=>{
                // println!("next limiter!");
                limiter
            }
            None=>{
                let lim = DirectRateLimiter::<LeakyBucket>::per_second(NonZeroU32::new(1).unwrap());
                // println!("first limiter!");
                self.hash.insert(id.clone(), lim);
                self.hash.get(&id).unwrap()
            }
        };
        {
            let mut lim = lim.clone();
            let ratelimit_future = Ratelimit::new(&mut lim);
            ratelimit_future.compat().await.expect("ratelimit_future unknown error");
        };
        Ok(())
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use async_std::{task};

    #[test]
    fn no_start_urls_finished() {


        let res = task::block_on(async {

            
            let limiter = Limiter::new();

            

            let c = || async move {
                let mut futs = Vec::new();
                futs.push(limiter.limit("domain.com"));
                futs.push(limiter.limit("domain.com"));
                futs.push(limiter.limit("domain.com"));
                futs.push(limiter.limit("domain.com"));
                futs.push(limiter.limit("domain.com"));
                futs.push(limiter.limit("domain.com"));

                println!("before limit");
                let start = Instant::now();
                join_all(futs).await;
                let duration = start.elapsed();
                println!("after limit {:?}", duration);
            };
            c().await;

        });
        assert!(true);
    }
}