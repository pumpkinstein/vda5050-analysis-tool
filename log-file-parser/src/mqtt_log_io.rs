//! Contains the logic for file I/O, specifically the custom iterator for splitting VDA 5050 log files.
//! Log file must have been created by MQTT client "mosquitto" by subscribing to all topics
use bstr::ByteSlice;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_while},
    combinator::map_res,
};

/// An iterator that splits a byte slice containing VDA 5050 log data into individual message records.
///
/// VDA 5050 logs can be multiline and don't have a consistent line-based separator. However,
/// each VDA 5050 MQTT message topic starts with the byte pattern known as root topic. E.g. `uagv/v1/`.
/// This iterator uses that pattern as a delimiter to find the start of each message.
pub struct VdaIterator<'a> {
    /// The full data slice to iterate over.
    data: &'a [u8],
    /// A boxed iterator that yields the starting indices of all occurrences of the delimiter.
    /// We use a trait object because the concrete type is not easily nameable.
    indices: Box<dyn Iterator<Item = usize> + 'a>,
    /// The starting index of the *current* message record that `next()` will return.
    current_match_start: Option<usize>,
}

impl<'a> VdaIterator<'a> {
    /// Creates a new `VdaIterator` for the given data slice.
    pub fn new(data: &'a [u8]) -> Self {
        // The delimiter that signifies the start of a VDA 5050 message.
        // TOOD: pass this as argument
        const VDA5050_DELIMITER: &[u8] = b"uagv/v1/";

        let mut indices = Box::new(data.find_iter(VDA5050_DELIMITER));

        // The first call to `indices.next()` finds the start of the very first record.
        let first_match = indices.next();

        Self {
            data,
            indices,
            current_match_start: first_match,
        }
    }
}

impl<'a> Iterator for VdaIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        // If `current_match_start` is None, it means we've either exhausted the iterator
        // or there were no matches to begin with.
        let start = self.current_match_start?;

        // Find the start of the *next* record. This will be the end of our *current* record.
        let next_match = self.indices.next();

        // The next call to `next()` will start from this new position.
        self.current_match_start = next_match;

        // If `next_match` is `None`, it means we're at the last record in the file.
        // In that case, the record extends to the end of the data slice.
        let end = next_match.unwrap_or(self.data.len());

        // The resulting slice is the complete VDA 5050 message record.
        Some(&self.data[start..end])
    }
}

/// A temporary struct to hold the fields parsed from the MQTT topic.
pub struct Topic<'a> {
    pub manufacturer: &'a str,
    pub serial_number: &'a str,
    pub msg_type: &'a str,
}

/// Uses `nom` to parse the VDA 5050 topic prefix from a raw log entry slice.
pub fn parse_topic<'a>(input: &'a [u8]) -> IResult<&'a [u8], Topic<'a>> {
    let not_separator = |c: u8| c != b'/' && c != b' ';

    let (rest, (manufacturer, _, serial_number, _, msg_type, _)) = (
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b" "[..]), // The space separating the topic from the JSON payload.
    )
        .parse(input)?;

    let topic = Topic {
        manufacturer,
        serial_number,
        msg_type,
    };

    Ok((rest, topic))
}

/// Parses a SemVer string (e.g., "2.0.1") into a packed u32 for efficient comparison and storage.
pub fn parse_version(version: &str) -> u32 {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    (major as u32) << 24 | (minor as u32) << 16 | (patch as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vda_iterator_simple() {
        let log_data =
            b"some preamble...uagv/v1/first_msg...some trailing data...uagv/v1/second_msg...end";
        let mut iterator = VdaIterator::new(log_data);

        assert_eq!(
            iterator.next(),
            Some(&b"uagv/v1/first_msg...some trailing data..."[..])
        );
        assert_eq!(iterator.next(), Some(&b"uagv/v1/second_msg...end"[..]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_vda_iterator_no_matches() {
        let log_data = b"no vda messages here";
        let mut iterator = VdaIterator::new(log_data);
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_vda_iterator_starts_with_match() {
        let log_data = b"uagv/v1/first...uagv/v1/second";
        let mut iterator = VdaIterator::new(log_data);
        assert_eq!(iterator.next(), Some(&b"uagv/v1/first..."[..]));
        assert_eq!(iterator.next(), Some(&b"uagv/v1/second"[..]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_vda_iterator_empty_input() {
        let log_data = b"";
        let mut iterator = VdaIterator::new(log_data);
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("2.1.0"), (2 << 24) | (1 << 16) | 0);
        assert_eq!(parse_version("1.0.0"), (1 << 24));
        assert_eq!(parse_version("0.0.0"), 0);
        assert_eq!(parse_version("invalid"), 0);
    }
}
