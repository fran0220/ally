use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

static EPISODE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^第([一二三四五六七八九十百千\d]+)集[：:\s]*(.*)?")
        .expect("episode marker regex must compile")
});
static CHAPTER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^第([一二三四五六七八九十百千\d]+)章[：:\s]*(.*)?")
        .expect("chapter marker regex must compile")
});
static ACT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^第([一二三四五六七八九十百千\d]+)幕[：:\s]*(.*)?")
        .expect("act marker regex must compile")
});
static ROUND_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^第([一二三四五六七八九十百千\d]+)回[：:\s]*(.*)?")
        .expect("round marker regex must compile")
});
static DIALOG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^第([一二三四五六七八九十百千\d]+)话[：:\s]*(.*)?")
        .expect("dialog marker regex must compile")
});
static SCENE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(\d+)-\d+[【\[](.*?)[】\]]").expect("scene marker regex must compile")
});
static NUMBERED_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(\d+)[\.、：:]\s*(.+)").expect("numbered marker regex must compile")
});
static NUMBERED_ESCAPED_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(\d+)\\\.\s*(.+)").expect("escaped numbered marker regex must compile")
});
static NUMBERED_DIRECT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|\n\n)(\d+)([一-龥])").expect("direct numbered marker regex must compile")
});
static EPISODE_EN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?im)^Episode\s*(\d+)[：:\s]*(.*)?")
        .expect("english episode marker regex must compile")
});
static CHAPTER_EN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?im)^Chapter\s*(\d+)[：:\s]*(.*)?")
        .expect("english chapter marker regex must compile")
});
static BOLD_NUMBER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\*\*(\d+)\*\*").expect("bold number marker regex must compile"));
static PURE_NUMBER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^(\d+)\s*$").expect("pure number marker regex must compile"));

static MARKER_PREFIX_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(?:第[一二三四五六七八九十百千\d]+[集章幕回话]|Episode\s*\d+|Chapter\s*\d+|\*\*\d+\*\*|\d+)[\.、：:\s]*",
    )
    .expect("marker prefix regex must compile")
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeMarkerMatch {
    pub index: usize,
    pub text: String,
    pub episode_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSplit {
    pub number: i32,
    pub title: String,
    pub word_count: usize,
    pub start_index: usize,
    pub end_index: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeMarkerConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeMarkerResult {
    pub has_markers: bool,
    pub marker_type: String,
    pub marker_type_key: String,
    pub confidence: EpisodeMarkerConfidence,
    pub matches: Vec<EpisodeMarkerMatch>,
    pub preview_splits: Vec<PreviewSplit>,
}

impl Default for EpisodeMarkerResult {
    fn default() -> Self {
        Self {
            has_markers: false,
            marker_type: String::new(),
            marker_type_key: String::new(),
            confidence: EpisodeMarkerConfidence::Low,
            matches: Vec::new(),
            preview_splits: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeSplitChunk {
    pub number: i32,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub word_count: usize,
}

#[derive(Clone, Copy)]
enum NumberExtractor {
    Chinese,
    Arabic,
}

#[derive(Clone, Copy)]
struct DetectionPattern<'a> {
    regex: &'a Regex,
    type_key: &'a str,
    type_name: &'a str,
    number_extractor: NumberExtractor,
    dedupe_same_episode: bool,
}

pub fn chinese_to_number(chinese: &str) -> i32 {
    if chinese.chars().all(|item| item.is_ascii_digit()) {
        return chinese.parse::<i32>().unwrap_or(0);
    }

    let mut result = 0_i32;
    let mut temp = 0_i32;
    let mut last_unit = 1_i32;

    for ch in chinese.chars() {
        let Some(number) = chinese_number_value(ch) else {
            continue;
        };

        if number >= 10 {
            if temp == 0 {
                temp = 1;
            }
            temp *= number;
            if number >= last_unit {
                result += temp;
                temp = 0;
            }
            last_unit = number;
        } else {
            temp = number;
        }
    }

    result + temp
}

pub fn detect_episode_markers(content: &str) -> EpisodeMarkerResult {
    let mut result = EpisodeMarkerResult::default();
    if content.chars().count() < 100 {
        return result;
    }

    for pattern in detection_patterns() {
        let mut matches: Vec<EpisodeMarkerMatch> = Vec::new();

        for captures in pattern.regex.captures_iter(content) {
            let Some(found) = captures.get(0) else {
                continue;
            };
            let Some(episode_number) = extract_episode_number(&captures, pattern.number_extractor)
            else {
                continue;
            };

            if pattern.dedupe_same_episode
                && matches
                    .iter()
                    .any(|item| item.episode_number == episode_number)
            {
                continue;
            }

            matches.push(EpisodeMarkerMatch {
                index: found.start(),
                text: found.as_str().to_string(),
                episode_number,
            });
        }

        if matches.len() >= 2 && matches.len() > result.matches.len() {
            result.matches = matches;
            result.marker_type = pattern.type_name.to_string();
            result.marker_type_key = pattern.type_key.to_string();
            result.has_markers = true;
        }
    }

    if !result.has_markers {
        return result;
    }

    result
        .matches
        .sort_by(|left, right| left.index.cmp(&right.index));
    result.confidence = detect_confidence(content, &result.matches);
    result.preview_splits = build_preview_splits(content, &result.matches);

    result
}

pub fn split_by_markers(
    content: &str,
    marker_result: &EpisodeMarkerResult,
) -> Vec<EpisodeSplitChunk> {
    if !marker_result.has_markers || marker_result.preview_splits.is_empty() {
        return Vec::new();
    }

    marker_result
        .preview_splits
        .iter()
        .map(|split| {
            let episode_content = content[split.start_index..split.end_index]
                .trim()
                .to_string();
            let title = if split.title.trim().is_empty() {
                format!("第 {} 集", split.number)
            } else {
                split.title.clone()
            };

            EpisodeSplitChunk {
                number: split.number,
                title,
                summary: String::new(),
                word_count: count_words_like_word(&episode_content),
                content: episode_content,
            }
        })
        .collect::<Vec<EpisodeSplitChunk>>()
}

pub fn count_words_like_word(text: &str) -> usize {
    let mut english_words = 0_usize;
    let mut in_english = false;
    let mut chinese_chars = 0_usize;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if !in_english {
                english_words += 1;
                in_english = true;
            }
            continue;
        }
        in_english = false;

        let code = ch as u32;
        if (0x4e00..=0x9fa5).contains(&code)
            || (0x3400..=0x4dbf).contains(&code)
            || (0x20000..=0x2a6df).contains(&code)
        {
            chinese_chars += 1;
        }
    }

    english_words + chinese_chars
}

fn detection_patterns() -> [DetectionPattern<'static>; 13] {
    [
        DetectionPattern {
            regex: &EPISODE_PATTERN,
            type_key: "episode",
            type_name: "第X集",
            number_extractor: NumberExtractor::Chinese,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &CHAPTER_PATTERN,
            type_key: "chapter",
            type_name: "第X章",
            number_extractor: NumberExtractor::Chinese,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &ACT_PATTERN,
            type_key: "act",
            type_name: "第X幕",
            number_extractor: NumberExtractor::Chinese,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &ROUND_PATTERN,
            type_key: "round",
            type_name: "第X回",
            number_extractor: NumberExtractor::Chinese,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &DIALOG_PATTERN,
            type_key: "dialog",
            type_name: "第X话",
            number_extractor: NumberExtractor::Chinese,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &SCENE_PATTERN,
            type_key: "scene",
            type_name: "X-Y【场景】",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: true,
        },
        DetectionPattern {
            regex: &NUMBERED_PATTERN,
            type_key: "numbered",
            type_name: "数字编号",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &NUMBERED_ESCAPED_PATTERN,
            type_key: "numberedEscaped",
            type_name: "数字编号(转义)",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &NUMBERED_DIRECT_PATTERN,
            type_key: "numberedDirect",
            type_name: "数字+中文",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &EPISODE_EN_PATTERN,
            type_key: "episodeEn",
            type_name: "Episode X",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &CHAPTER_EN_PATTERN,
            type_key: "chapterEn",
            type_name: "Chapter X",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &BOLD_NUMBER_PATTERN,
            type_key: "boldNumber",
            type_name: "**数字**",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
        DetectionPattern {
            regex: &PURE_NUMBER_PATTERN,
            type_key: "pureNumber",
            type_name: "纯数字",
            number_extractor: NumberExtractor::Arabic,
            dedupe_same_episode: false,
        },
    ]
}

fn chinese_number_value(ch: char) -> Option<i32> {
    match ch {
        '零' | '〇' => Some(0),
        '一' | '壹' => Some(1),
        '二' | '贰' | '两' => Some(2),
        '三' | '叁' => Some(3),
        '四' | '肆' => Some(4),
        '五' | '伍' => Some(5),
        '六' | '陆' => Some(6),
        '七' | '柒' => Some(7),
        '八' | '捌' => Some(8),
        '九' | '玖' => Some(9),
        '十' | '拾' => Some(10),
        '百' | '佰' => Some(100),
        '千' | '仟' => Some(1000),
        _ => None,
    }
}

fn extract_episode_number(captures: &Captures<'_>, extractor: NumberExtractor) -> Option<i32> {
    let raw = captures.get(1)?.as_str();
    match extractor {
        NumberExtractor::Chinese => Some(chinese_to_number(raw)),
        NumberExtractor::Arabic => raw.parse::<i32>().ok(),
    }
}

fn detect_confidence(content: &str, matches: &[EpisodeMarkerMatch]) -> EpisodeMarkerConfidence {
    let match_count = matches.len();
    if match_count <= 1 {
        return EpisodeMarkerConfidence::Low;
    }

    let first = &matches[0];
    let last = &matches[match_count - 1];
    let total_distance = content[first.index..last.index].chars().count() as f64;
    let avg_distance = total_distance / ((match_count - 1) as f64);

    if match_count >= 3 && (500.0..=8000.0).contains(&avg_distance) {
        EpisodeMarkerConfidence::High
    } else if match_count >= 2 {
        EpisodeMarkerConfidence::Medium
    } else {
        EpisodeMarkerConfidence::Low
    }
}

fn build_preview_splits(content: &str, matches: &[EpisodeMarkerMatch]) -> Vec<PreviewSplit> {
    let mut preview_splits: Vec<PreviewSplit> = Vec::new();

    let first_match = &matches[0];
    if first_match.episode_number > 1 && content[..first_match.index].chars().count() > 100 {
        let episode_content = &content[0..first_match.index];
        preview_splits.push(PreviewSplit {
            number: 1,
            title: "第 1 集".to_string(),
            word_count: count_words_like_word(episode_content),
            start_index: 0,
            end_index: first_match.index,
            preview: build_preview(episode_content),
        });
    }

    for (idx, marker) in matches.iter().enumerate() {
        let start_index = if idx == 0 && preview_splits.is_empty() {
            0
        } else {
            marker.index
        };
        let end_index = if idx + 1 < matches.len() {
            matches[idx + 1].index
        } else {
            content.len()
        };

        let episode_content = &content[start_index..end_index];
        let marker_position = marker.index.saturating_sub(start_index);
        let prefix_length = marker_prefix_len(&marker.text)
            .filter(|item| *item > 0)
            .unwrap_or(marker.text.len());
        let preview_start = marker_position.saturating_add(prefix_length);
        let preview_source = if preview_start < episode_content.len() {
            &episode_content[preview_start..]
        } else {
            ""
        };

        preview_splits.push(PreviewSplit {
            number: marker.episode_number,
            title: format!("第 {} 集", marker.episode_number),
            word_count: count_words_like_word(episode_content),
            start_index,
            end_index,
            preview: build_preview(preview_source),
        });
    }

    preview_splits
}

fn marker_prefix_len(marker_text: &str) -> Option<usize> {
    MARKER_PREFIX_PATTERN
        .captures(marker_text)
        .and_then(|captures| captures.get(0))
        .map(|matched| matched.as_str().len())
}

fn build_preview(source: &str) -> String {
    let first_window = source.chars().take(50).collect::<String>();
    let trimmed = first_window.trim();
    let preview = trimmed.chars().take(20).collect::<String>();
    if preview.chars().count() >= 20 {
        format!("{preview}...")
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EpisodeMarkerConfidence, chinese_to_number, count_words_like_word, detect_episode_markers,
        split_by_markers,
    };

    #[test]
    fn chinese_to_number_supports_common_forms() {
        assert_eq!(chinese_to_number("十"), 10);
        assert_eq!(chinese_to_number("十一"), 11);
        assert_eq!(chinese_to_number("二十"), 20);
        assert_eq!(chinese_to_number("两百三"), 203);
        assert_eq!(chinese_to_number("一百二十三"), 103);
    }

    #[test]
    fn count_words_like_word_matches_frontend_behavior() {
        assert_eq!(count_words_like_word("hello world 你好"), 4);
        assert_eq!(count_words_like_word("alpha123 beta 你 好"), 4);
    }

    #[test]
    fn detect_episode_markers_returns_empty_for_short_content() {
        let result = detect_episode_markers("第1集\n简介");
        assert!(!result.has_markers);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn detect_episode_markers_with_high_confidence() {
        let filler = "剧情".repeat(300);
        let content =
            format!("第1集 开场\n{filler}\n\n第2集 冲突\n{filler}\n\n第3集 结尾\n{filler}");

        let result = detect_episode_markers(&content);
        assert!(result.has_markers);
        assert_eq!(result.marker_type_key, "episode");
        assert_eq!(result.matches.len(), 3);
        assert_eq!(result.confidence, EpisodeMarkerConfidence::High);
        assert_eq!(result.preview_splits.len(), 3);
        assert_eq!(result.preview_splits[0].number, 1);
        assert!(result.preview_splits[0].word_count > 0);
    }

    #[test]
    fn detect_episode_markers_supports_round_markers() {
        let filler = "内容".repeat(220);
        let content = format!("第1回 序\n{filler}\n\n第2回 承\n{filler}");

        let result = detect_episode_markers(&content);
        assert!(result.has_markers);
        assert_eq!(result.marker_type_key, "round");
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.preview_splits[0].number, 1);
    }

    #[test]
    fn detect_episode_markers_backfills_missing_first_episode() {
        let prologue = "序章".repeat(120);
        let filler = "剧情".repeat(180);
        let content = format!("{prologue}\n\n第3集 起\n{filler}\n\n第4集 承\n{filler}");

        let result = detect_episode_markers(&content);
        assert!(result.has_markers);
        assert!(result.preview_splits.len() >= 3);
        assert_eq!(result.preview_splits[0].number, 1);
        assert_eq!(result.preview_splits[0].start_index, 0);
        assert!(result.preview_splits[0].end_index > 100);
    }

    #[test]
    fn split_by_markers_builds_episode_chunks() {
        let filler = "内容".repeat(180);
        let content = format!("1. 开始{filler}\n\n2. 发展{filler}");
        let marker_result = detect_episode_markers(&content);

        assert!(marker_result.has_markers);
        let chunks = split_by_markers(&content, &marker_result);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].number, 1);
        assert_eq!(chunks[0].title, "第 1 集");
        assert_eq!(chunks[0].summary, "");
        assert!(chunks[0].content.contains("开始"));
        assert_eq!(
            chunks[0].word_count,
            count_words_like_word(&chunks[0].content)
        );
    }
}
