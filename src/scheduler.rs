//! This module implements the "universal ping algorithm" for determining when
//! to next poll the user as to what they're doing. The scheduling algorithm
//! used here ensures that, given two target intervals between pings, a and b,
//! the ping times for a will be a superset of those for b iff a < b. more info
//! about this algorithm can be found on [the Beeminder forums].
//!
//! [the Beeminder forums]: https://forum.beeminder.com/t/possible-new-tagtime-universal-ping-algorithm/4143/31

use chrono::{DateTime, TimeZone, Utc};
use rug::Integer;
use std::iter::Iterator;

const IA: i64 = 3125;
const IM: i64 = 34359738337;
const GAP: i64 = 45 * 60;
const SEED: i64 = 20180809;

/// A linear congruence generator, whose offset (increment) is 0.
#[derive(Debug, Clone)]
pub struct LCG {
    pub multiplier: Integer,
    pub modulus: Integer,
    pub state: Integer,
}

impl LCG {
    pub fn pow(&mut self, exp: Integer) {
        let multiplier = self
            .multiplier
            .clone()
            .pow_mod(&exp, &self.modulus)
            .unwrap();
        self.state = (multiplier * &self.state) % &self.modulus;
    }

    pub fn next(&mut self) {
        self.pow(Integer::from(1))
    }
}

impl Default for LCG {
    fn default() -> Self {
        LCG {
            multiplier: IA.into(),
            modulus: IM.into(),
            state: SEED.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    time: DateTime<Utc>,
    /// Desired average gap length in seconds
    gap: Integer,
    lcg: LCG,
}

impl State {
    pub fn new(time: DateTime<Utc>, gap: Integer, lcg: LCG) -> State {
        State { time, gap, lcg }
    }

    pub fn from_millis(n: i64) -> State {
        let mut s = State::default();
        s.time = Utc.timestamp_millis(n);
        s
    }

    pub fn lcg(&self) -> &LCG {
        &self.lcg
    }

    pub fn next_time(&mut self, cur: DateTime<Utc>) {
        if cur >= self.time {
            let threshold = &self.lcg.modulus / (&self.gap * Integer::from(10));

            let prev_incs = self.time.timestamp_millis() / 100;
            let cur_incs = cur.timestamp_millis() / 100;
            let mut new_incs = cur_incs + 1;

            if cur_incs > prev_incs {
                self.lcg.pow(Integer::from(cur_incs - prev_incs));
            }

            while {
                self.lcg.next();
                &self.lcg.state
            } >= &threshold
            {
                new_incs += 1;
            }

            self.time = Utc.timestamp_millis(new_incs * 100);
        }
    }
}

impl Iterator for State {
    type Item = DateTime<Utc>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_time(self.time);
        Some(self.time)
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            time: Utc::now(),
            gap: Integer::from(GAP),
            lcg: LCG::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{State, LCG};
    use chrono::{offset::TimeZone, Utc};
    use rug::Integer;

    const INIT_TIME: i64 = 1533812000000;
    const IA: i64 = 3125;
    const IM: i64 = 34359738337;
    const GAP: i64 = 45 * 60;
    const SEED: i64 = 20180809;

    const IDEAL: [i64; 4] =
        [15338123839, 15338127440, 15338175871, 15338193911];
    const IDEAL_RNG: [i64; 4] =
        [28705289788, 25113527930, 2132419542, 32381569709];

    fn create_lcg() -> LCG {
        LCG {
            multiplier: IA.into(),
            modulus: IM.into(),
            state: SEED.into(),
        }
    }

    fn create_state() -> State {
        let lcg = create_lcg();

        State {
            time: Utc.timestamp_millis(INIT_TIME),
            gap: Integer::from(GAP),
            lcg,
        }
    }

    #[test]
    fn test_correct_times() {
        let s = create_state();

        assert_eq!(
            s.take(4)
                .map(|x| x.timestamp_millis() / 100)
                .collect::<Vec<_>>(),
            IDEAL.to_vec()
        );
    }

    #[test]
    fn test_correct_times_ff() {
        let s = create_state();

        for i in 0..4 {
            let mut s = s.clone();
            assert_eq!(s.nth(i).unwrap().timestamp_millis() / 100, IDEAL[i]);
        }
    }

    #[test]
    fn test_correct_rng() {
        let mut lcg = create_lcg();

        for i in 0..4 {
            lcg.next();
            assert_eq!(lcg.state.to_i64().unwrap(), IDEAL_RNG[i]);
        }
    }

    #[test]
    fn test_correct_rng_pow() {
        let lcg = create_lcg();

        for i in 0..4 {
            let mut lcg = lcg.clone();
            lcg.pow(Integer::from(i + 1));
            assert_eq!(lcg.state.to_i64().unwrap(), IDEAL_RNG[i]);
        }
    }
}
