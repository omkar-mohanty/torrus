use num_traits::Num;
use std::ops::Range;

pub struct RangeExt<T: Num> {
    ranges: Vec<Range<T>>,
}

impl<T: Num + PartialOrd + Copy> RangeExt<T> {
    pub fn new(ranges: Vec<Range<T>>) -> Self {
        Self { ranges }
    }

    pub fn intersection(&self) -> Range<T> {
        let mut start = self.ranges[0].start;
        let mut end = self.ranges[0].end;

        for range in self.ranges.iter() {
            if range.start > start {
                start = range.start
            }

            if range.end < end {
                end = range.end
            }
        }

        Range { start, end }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::RangeExt;

    #[test]
    fn test_range_intersection() -> crate::Result<()> {
        let ranges = vec![Range { start: 0, end: 12 }, Range { start: 9, end: 15 }];

        let intersection = RangeExt::new(ranges).intersection();

        assert_eq!(Range { start: 9, end: 12 }, intersection);

        Ok(())
    }
}
