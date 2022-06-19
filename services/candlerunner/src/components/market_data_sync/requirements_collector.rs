use std::collections::HashMap;

use chrono::prelude::*;

use crate::models::instruments::Figi;

pub type Ranges = Vec<(DateTime<Utc>, DateTime<Utc>)>;

#[derive(Default)]
pub struct RequirementsCollector {
    instruments: HashMap<Figi, Ranges>,
}

fn merge_ranges(mut input: Ranges) -> Ranges {
    if input.is_empty() {
        return input;
    }

    input.sort_by_key(|elem| elem.0);
    input
        .into_iter()
        .fold(Ranges::default(), |mut state, elem| {
            match state.last_mut() {
                None => {
                    state.push(elem);
                }
                Some(range) => {
                    if range.1 >= elem.0 {
                        range.1 = elem.1;
                    } else {
                        state.push(elem);
                    }
                }
            }

            state
        })
}

impl RequirementsCollector {
    pub fn push(&mut self, figi: Figi, time_from: DateTime<Utc>, time_to: Option<DateTime<Utc>>) {
        let range = (time_from, time_to.unwrap_or_else(Utc::now));

        let ranges = self.instruments.entry(figi).or_default();
        ranges.push(range);
    }

    pub fn finalize(mut self) -> HashMap<Figi, Ranges> {
        let mut res: HashMap<Figi, Ranges> = Default::default();

        for (figi, raw_ranges) in self.instruments.drain() {
            res.insert(figi, merge_ranges(raw_ranges));
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::merge_ranges;
    use chrono::{Duration, Utc};

    #[test]
    fn test_merge_ranges() {
        let base_time = Utc::now();
        let second = Duration::seconds(1);

        let range = vec![
            (base_time + second * 32, base_time + second * 33),
            (base_time + second * 40, base_time + second * 45),
            (base_time, base_time + second * 20),
            (base_time + second * 10, base_time + second * 30),
            (base_time + second * 21, base_time + second * 31),
            (base_time + second * 43, base_time + second * 50),
        ];

        let expected = vec![
            (base_time, base_time + second * 31),
            (base_time + second * 32, base_time + second * 33),
            (base_time + second * 40, base_time + second * 50),
        ];

        let merged = merge_ranges(range);

        assert_eq!(merged, expected);
    }
}
