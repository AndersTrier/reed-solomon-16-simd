use crate::rate::DecoderWork;
use std::mem::take;

// ======================================================================
// DecoderResult - PUBLIC

/// Result of decoding. Contains the restored original shards.
///
/// This struct is created by [`ReedSolomonDecoder::decode`]
/// and [`RateDecoder::decode`].
///
/// [`RateDecoder::decode`]: crate::rate::RateDecoder::decode
/// [`ReedSolomonDecoder::decode`]: crate::ReedSolomonDecoder::decode
pub struct DecoderResult<'a> {
    work: &'a mut DecoderWork,
}

impl<'a> DecoderResult<'a> {
    /// Returns restored original shard with given `index`
    /// or `None` if given `index` doesn't correspond to
    /// a missing original shard.
    pub fn restored_original(&self, index: usize) -> Option<&[u8]> {
        self.work.restored_original(index)
    }

    /// Returns iterator over all restored original shards
    /// and their indexes, ordered by indexes.
    pub fn restored_original_iter(&self) -> RestoredOriginal {
        RestoredOriginal::new(self.work)
    }

    /// Returns inner Vec
    pub fn into_inner_vec(self) -> Vec<u8> {
        let data = take(&mut self.work.shards.data);
        //TODO: truncate
        //data.truncate(self.
        self.work.reset_received();
        data
    }
}

// ======================================================================
// DecoderResult - CRATE

impl<'a> DecoderResult<'a> {
    pub(crate) fn new(work: &'a mut DecoderWork) -> Self {
        Self { work }
    }
}

// ======================================================================
// DecoderResult - IMPL DROP

impl<'a> Drop for DecoderResult<'a> {
    fn drop(&mut self) {
        self.work.reset_received();
    }
}

// ======================================================================
// RestoredOriginal - PUBLIC

/// Iterator over restored original shards and their indexes.
///
/// This struct is created by [`DecoderResult::restored_original_iter`].
pub struct RestoredOriginal<'a> {
    ended: bool,
    next_index: usize,
    work: &'a DecoderWork,
}

// ======================================================================
// RestoredOriginal - IMPL Iterator

impl<'a> Iterator for RestoredOriginal<'a> {
    type Item = (usize, &'a [u8]);
    fn next(&mut self) -> Option<(usize, &'a [u8])> {
        if self.ended {
            None
        } else {
            let mut index = self.next_index;
            while index < self.work.original_count() {
                if let Some(original) = self.work.restored_original(index) {
                    self.next_index = index + 1;
                    return Some((index, original));
                }
                index += 1
            }
            self.ended = true;
            None
        }
    }
}

// ======================================================================
// RestoredOriginal - CRATE

impl<'a> RestoredOriginal<'a> {
    pub(crate) fn new(work: &'a DecoderWork) -> Self {
        Self {
            ended: false,
            next_index: 0,
            work,
        }
    }
}

// ======================================================================
// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_util, ReedSolomonDecoder, ReedSolomonEncoder};

    #[test]
    // DecoderResult::restored_original
    // DecoderResult::restored_original_iter
    // RestoredOriginal
    fn decoder_result() {
        let original = test_util::generate_original(3, 1024, 0);

        let mut encoder = ReedSolomonEncoder::new(3, 2, 1024).unwrap();
        let mut decoder = ReedSolomonDecoder::new(3, 2, 1024).unwrap();

        for original in &original {
            encoder.add_original_shard(original).unwrap();
        }

        let result = encoder.encode().unwrap();
        let recovery: Vec<_> = result.recovery_iter().collect();

        decoder.add_original_shard(1, &original[1]).unwrap();
        decoder.add_recovery_shard(0, recovery[0]).unwrap();
        decoder.add_recovery_shard(1, recovery[1]).unwrap();

        let result: DecoderResult = decoder.decode().unwrap();

        assert_eq!(result.restored_original(0).unwrap(), original[0]);
        assert!(result.restored_original(1).is_none());
        assert_eq!(result.restored_original(2).unwrap(), original[2]);
        assert!(result.restored_original(3).is_none());

        let mut iter: RestoredOriginal = result.restored_original_iter();
        assert_eq!(iter.next(), Some((0, original[0].as_slice())));
        assert_eq!(iter.next(), Some((2, original[2].as_slice())));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}
