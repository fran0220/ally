use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SrtEntry {
    pub index: i32,
    pub start_time: String,
    pub end_time: String,
    pub text: String,
}

pub fn parse_srt(text: &str) -> Vec<SrtEntry> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in trimmed.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current);
                current = Vec::new();
            }
            continue;
        }
        current.push(line);
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    let mut entries: Vec<SrtEntry> = Vec::new();
    for block in blocks {
        if block.len() < 3 {
            continue;
        }

        let index = match block[0].trim().parse::<i32>() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let (start_time, end_time) = match parse_time_line(block[1]) {
            Some(value) => value,
            None => continue,
        };
        let text = block[2..].join("\n");

        entries.push(SrtEntry {
            index,
            start_time,
            end_time,
            text,
        });
    }

    entries
}

pub fn format_srt_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "00:00:00,000".to_string();
    }

    let scaled = (seconds * 1000.0).round();
    let total_millis = if scaled <= 0.0 {
        0_i64
    } else if scaled >= i64::MAX as f64 {
        i64::MAX
    } else {
        scaled as i64
    };

    let hours = total_millis / 3_600_000;
    let minutes = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1_000;
    let millis = total_millis % 1_000;

    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

pub fn parse_srt_time(time_str: &str) -> f64 {
    let mut parts = time_str.trim().split(':');
    let Some(hours_raw) = parts.next() else {
        return 0.0;
    };
    let Some(minutes_raw) = parts.next() else {
        return 0.0;
    };
    let Some(seconds_and_millis_raw) = parts.next() else {
        return 0.0;
    };
    if parts.next().is_some() {
        return 0.0;
    }

    let (seconds_raw, millis_raw) = match seconds_and_millis_raw
        .split_once(',')
        .or_else(|| seconds_and_millis_raw.split_once('.'))
    {
        Some(value) => value,
        None => return 0.0,
    };

    let hours = match hours_raw.parse::<f64>() {
        Ok(value) => value,
        Err(_) => return 0.0,
    };
    let minutes = match minutes_raw.parse::<f64>() {
        Ok(value) => value,
        Err(_) => return 0.0,
    };
    let seconds = match seconds_raw.parse::<f64>() {
        Ok(value) => value,
        Err(_) => return 0.0,
    };
    let millis = match millis_raw.parse::<f64>() {
        Ok(value) => value,
        Err(_) => return 0.0,
    };

    (hours * 3600.0) + (minutes * 60.0) + seconds + (millis / 1000.0)
}

pub fn srt_entries_to_string(entries: &[SrtEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}\n{} --> {}\n{}",
                entry.index, entry.start_time, entry.end_time, entry.text
            )
        })
        .collect::<Vec<String>>()
        .join("\n\n")
}

fn parse_time_line(line: &str) -> Option<(String, String)> {
    let arrow_index = line.find("-->")?;
    let start_raw = line.get(..arrow_index)?.split_whitespace().next()?;
    let end_raw = line.get(arrow_index + 3..)?.split_whitespace().next()?;

    if start_raw.is_empty() || end_raw.is_empty() {
        return None;
    }

    Some((start_raw.to_string(), end_raw.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{SrtEntry, format_srt_time, parse_srt, parse_srt_time, srt_entries_to_string};

    #[test]
    fn parse_srt_parses_entries_with_multiline_text() {
        let text = "1\n00:00:00,000 --> 00:00:02,500\n第一行\n第二行\n\n2\n00:00:03,000 --> 00:00:04,250\nHello world";
        let entries = parse_srt(text);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[0].start_time, "00:00:00,000");
        assert_eq!(entries[0].end_time, "00:00:02,500");
        assert_eq!(entries[0].text, "第一行\n第二行");
        assert_eq!(entries[1].index, 2);
    }

    #[test]
    fn parse_srt_supports_windows_line_endings_and_skips_invalid_blocks() {
        let text = "1\r\n00:00:00,000 --> 00:00:01,000\r\nline\r\n\r\ninvalid\r\n00:00:01,000 -->\r\nmissing\r\n\r\n2\r\n00:00:01,500 --> 00:00:02,000\r\nnext";
        let entries = parse_srt(text);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[1].index, 2);
    }

    #[test]
    fn parse_and_format_srt_time_roundtrip() {
        let seconds = parse_srt_time("01:02:03,045");
        assert!((seconds - 3723.045).abs() < 1e-9);
        assert!((parse_srt_time("00:00:10.500") - 10.5).abs() < 1e-9);
        assert_eq!(parse_srt_time("invalid"), 0.0);

        assert_eq!(format_srt_time(3723.045), "01:02:03,045");
        assert_eq!(format_srt_time(-1.0), "00:00:00,000");
    }

    #[test]
    fn srt_entries_to_string_serializes_entries() {
        let entries = vec![
            SrtEntry {
                index: 1,
                start_time: "00:00:00,000".to_string(),
                end_time: "00:00:01,000".to_string(),
                text: "alpha".to_string(),
            },
            SrtEntry {
                index: 2,
                start_time: "00:00:01,500".to_string(),
                end_time: "00:00:03,000".to_string(),
                text: "beta\ngamma".to_string(),
            },
        ];

        let raw = srt_entries_to_string(&entries);
        assert_eq!(
            raw,
            "1\n00:00:00,000 --> 00:00:01,000\nalpha\n\n2\n00:00:01,500 --> 00:00:03,000\nbeta\ngamma"
        );
    }
}
