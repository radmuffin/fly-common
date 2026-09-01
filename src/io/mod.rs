use serde::de::DeserializeOwned;
use std::io::Read;

/// Batch execution helper to chunk large collections of items inside SQLite transactions.
pub fn chunk_slice<T>(items: &[T], chunk_size: usize) -> impl Iterator<Item = &[T]> {
    items.chunks(chunk_size)
}

/// Parses a CSV data stream into strongly-typed records.
pub fn parse_csv<R: Read, T: DeserializeOwned>(reader: R) -> Result<Vec<T>, String> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(reader);

    let mut records = Vec::new();
    for result in csv_reader.deserialize() {
        match result {
            Ok(record) => records.push(record),
            Err(e) => return Err(format!("CSV deserialization error: {}", e)),
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct PlaceRecord {
        name: String,
        category: String,
    }

    #[test]
    fn test_csv_parser() {
        let csv_data = "name,category\nTokyo Tower,Sightseeing\nIchiran Ramen,Food\n";
        let records: Vec<PlaceRecord> = parse_csv(csv_data.as_bytes()).expect("parse csv");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "Tokyo Tower");
        assert_eq!(records[1].category, "Food");
    }

    #[test]
    fn test_chunk_slice() {
        let items = vec![1, 2, 3, 4, 5, 6, 7];
        let chunks: Vec<_> = chunk_slice(&items, 3).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], &[1, 2, 3]);
        assert_eq!(chunks[1], &[4, 5, 6]);
        assert_eq!(chunks[2], &[7]);
    }
}
