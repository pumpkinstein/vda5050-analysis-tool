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
/// each VDA 5050 MQTT message topic starts with the configured root topic. This iterator uses
/// that pattern as a delimiter to find the start of each message.
pub(crate) struct VdaIterator<'a> {
    /// The full data slice to iterate over.
    data: &'a [u8],
    /// The root topic, including its trailing slash, used as the message delimiter.
    delimiter: Vec<u8>,
    /// The starting index of the *current* message record that `next()` will return.
    current_match_start: Option<usize>,
}

impl<'a> VdaIterator<'a> {
    /// Creates a new `VdaIterator` for the given data slice and root-topic prefix.
    pub(crate) fn new(data: &'a [u8], root_topic_prefix: &[u8]) -> Self {
        let delimiter = root_topic_prefix.to_vec();

        // The first match finds the start of the very first record.
        let first_match = data.find(delimiter.as_slice());

        Self {
            data,
            delimiter,
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
        let search_start = start + self.delimiter.len();
        let next_match = self.data[search_start..]
            .find(self.delimiter.as_slice())
            .map(|offset| search_start + offset);

        // The next call to `next()` will start from this new position.
        self.current_match_start = next_match;

        // If `next_match` is `None`, it means we're at the last record in the file.
        // In that case, the record extends to the end of the data slice.
        let end = next_match.unwrap_or(self.data.len());

        // The resulting slice is the complete VDA 5050 message record.
        Some(&self.data[start..end])
    }
}

/// Converts a root topic such as `uagv/v1` into the prefix used in log records.
///
/// A trailing slash is accepted so values entered as either `uagv/v1` or `uagv/v1/`
/// behave identically.
pub(crate) fn root_topic_prefix(root_topic: &str) -> anyhow::Result<Vec<u8>> {
    let root_topic = root_topic.trim_end_matches('/');
    if root_topic.is_empty() {
        anyhow::bail!("Root topic must not be empty");
    }

    let mut prefix = root_topic.as_bytes().to_vec();
    prefix.push(b'/');
    Ok(prefix)
}

/// A temporary struct to hold the fields parsed from the MQTT topic.
pub(crate) struct Topic<'a> {
    pub manufacturer: &'a str,
    pub serial_number: &'a str,
    pub msg_type: &'a str,
}

/// Uses `nom` to parse the VDA 5050 topic prefix from a raw log entry slice.
pub(crate) fn parse_topic<'a>(input: &'a [u8]) -> IResult<&'a [u8], Topic<'a>> {
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
pub(crate) fn parse_version(version: &str) -> u32 {
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
        let mut iterator = VdaIterator::new(log_data, b"uagv/v1/");

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
        let mut iterator = VdaIterator::new(log_data, b"uagv/v1/");
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_vda_iterator_starts_with_match() {
        let log_data = b"uagv/v1/first...uagv/v1/second";
        let mut iterator = VdaIterator::new(log_data, b"uagv/v1/");
        assert_eq!(iterator.next(), Some(&b"uagv/v1/first..."[..]));
        assert_eq!(iterator.next(), Some(&b"uagv/v1/second"[..]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_vda_iterator_empty_input() {
        let log_data = b"";
        let mut iterator = VdaIterator::new(log_data, b"uagv/v1/");
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("2.1.0"), (2 << 24) | (1 << 16) | 0);
        assert_eq!(parse_version("1.0.0"), (1 << 24));
        assert_eq!(parse_version("0.0.0"), 0);
        assert_eq!(parse_version("invalid"), 0);
    }

    #[test]
    fn test_vda_iterator_custom_root_topic() {
        let log_data = b"preamble...fleet/v2/first...trailing...fleet/v2/second...end";
        let mut iterator = VdaIterator::new(log_data, b"fleet/v2/");

        assert_eq!(iterator.next(), Some(&b"fleet/v2/first...trailing..."[..]));
        assert_eq!(iterator.next(), Some(&b"fleet/v2/second...end"[..]));
        assert_eq!(iterator.next(), None);
    }
}
