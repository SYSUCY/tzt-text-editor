use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use std::{
    cmp::min,
    fmt::{self, Display},
    ops::{Deref, Range},
};
use crate::prelude::*;
use crate::editor::{AnnotatedString, Annotation};

mod graphemewidth;
use graphemewidth::GraphemeWidth;

mod textfragment;
use textfragment::TextFragment;

#[derive(Default, Clone)]
pub struct Line {
    fragments: Vec<TextFragment>, // fragments（文本片段向量）
    string: String, // string（字符串）
}

impl Line {
    // 通过字符串构建一个 Line 实例
    pub fn from(line_str: &str) -> Self {
        debug_assert!(line_str.is_empty() || line_str.lines().count() == 1);
        let fragments = Self::str_to_fragments(line_str);
        Self {
            fragments,
            string: String::from(line_str),
        }
    }

    // 字符串转换为文本片段的向量
    // 每个片段包含 grapheme（字素）、rendered_width（渲染宽度）、replacement（替代字符）、start（开始位置）
    fn str_to_fragments(line_str: &str) -> Vec<TextFragment> {
        line_str
            .grapheme_indices(true)
            .map(|(byte_idx, grapheme)| {
                let (replacement, rendered_width) = Self::get_replacement_character(grapheme)
                    .map_or_else(
                        || {
                            let unicode_width = grapheme.width();
                            let rendered_width = match unicode_width {
                                0 | 1 => GraphemeWidth::Half,
                                _ => GraphemeWidth::Full,
                            };
                            (None, rendered_width)
                        },
                        |replacement| (Some(replacement), GraphemeWidth::Half),
                    );

                TextFragment {
                    grapheme: grapheme.to_string(),
                    rendered_width,
                    replacement,
                    start: byte_idx,
                }
            })
            .collect()
    }
   
    fn rebuild_fragments(&mut self) {
        self.fragments = Self::str_to_fragments(&self.string);
    }

    // 根据输入字符串返回一个替代字符，用于表示特定的控制字符或空白字符
    fn get_replacement_character(for_str: &str) -> Option<char> {
        let width = for_str.width();
        match for_str {
            " " => None,
            "\t" => Some(' '),
            _ if width > 0 && for_str.trim().is_empty() => Some('␣'),
            _ if width == 0 => {
                let mut chars = for_str.chars();
                if let Some(ch) = chars.next() {
                    if ch.is_control() && chars.next().is_none() {
                        return Some('▯');
                    }
                }
                Some('·')
            }
            _ => None,
        }
    }

    // 获取给定列索引中可见的字素。
    // 请注意，列索引与字素索引不同：
    // 一个字素的宽度可以为 2 列。
    pub fn get_visible_graphemes(&self, range: Range<ColIdx>) -> String {
        self.get_annotated_visible_substr(range, None).to_string()
    }

    // 获取指定列范围内的带注解的字符串
    pub fn get_annotated_visible_substr(
        &self,
        range: Range<ColIdx>,
        annotations: Option<&Vec<Annotation>>,
    ) -> AnnotatedString {
        if range.start >= range.end {
            return AnnotatedString::default();
        }
    
        // 创建新的注解字符串
        let mut result = AnnotatedString::from(&self.string);
    
        // 应用注解
        if let Some(annotations) = annotations {
            for annotation in annotations {
                result.add_annotation(annotation.annotation_type, annotation.start, annotation.end);
            }
        }
    
        // 处理替代字符并截断
        let mut fragment_start = self.width();
        let mut truncate_left = false;
        let mut truncate_right = false;
        let mut replace_range = (None, None);
    
        for fragment in self.fragments.iter().rev() {
            let fragment_end = fragment_start;
            fragment_start = fragment_start.saturating_sub(fragment.rendered_width.into());
    
            if fragment_start > range.end {
                continue; // 没有到达可见范围，继续
            }
    
            if fragment_start < range.end && fragment_end > range.end {
                replace_range = (Some(fragment.start), None);
                truncate_right = true;
                break;
            } else if fragment_start == range.end {
                truncate_right = true;
                replace_range = (Some(fragment.start), None);
                break;
            }
    
            if fragment_end <= range.start {
                truncate_left = true;
                replace_range = (None, Some(fragment.start.saturating_add(fragment.grapheme.len())));
                break;
            } else if fragment_start < range.start && fragment_end > range.start {
                replace_range = (Some(0), Some(fragment.start.saturating_add(fragment.grapheme.len())));
                truncate_left = true;
                break;
            }
    
            if fragment_start >= range.start && fragment_end <= range.end {
                if let Some(replacement) = fragment.replacement {
                    let start = fragment.start;
                    let end = start.saturating_add(fragment.grapheme.len());
                    result.replace(start, end, &replacement.to_string());
                }
            }
        }
    
        if truncate_left {
            if let Some(end) = replace_range.1 {
                result.truncate_left_until(end);
            }
        }
        if truncate_right {
            if let Some(start) = replace_range.0 {
                result.truncate_right_from(start);
            } else {
                result.replace(self.string.len(), self.string.len(), "⋯");
            }
        } else if let Some(start) = replace_range.0 {
            result.replace(start, self.string.len(), "⋯");
        }
    
        result
    }

    //  返回行中的字素数量
    pub fn grapheme_count(&self) -> GraphemeIdx {
        self.fragments.len()
    }

    // 计算直到指定字素的列宽
    pub fn width_until(&self, grapheme_idx: GraphemeIdx) -> ColIdx {
        self.fragments
            .iter()
            .take(grapheme_idx)
            .map(|fragment| match fragment.rendered_width {
                GraphemeWidth::Half => 1,
                GraphemeWidth::Full => 2,
            })
            .sum()
    }

    // 返回整行的列宽
    pub fn width(&self) -> ColIdx {
        self.width_until(self.grapheme_count())
    }

    // 在指定字素索引处插入字符
    // 将一个字符插入到行中，或者如果 at == grapheme_count + 1，则将其附加到行尾
    pub fn insert_char(&mut self, character: char, at: GraphemeIdx) {
        debug_assert!(at.saturating_sub(1) <= self.grapheme_count());
        if let Some(fragment) = self.fragments.get(at) {
            self.string.insert(fragment.start, character);
        } else {
            self.string.push(character);
        }
        self.rebuild_fragments();
    }

    // 追加字符
    pub fn append_char(&mut self, character: char) {
        self.insert_char(character, self.grapheme_count());
    }

    // 删除指定字素索引处的字符
    pub fn delete(&mut self, at: GraphemeIdx) {
        debug_assert!(at <= self.grapheme_count());
        if let Some(fragment) = self.fragments.get(at) {
            let start = fragment.start;
            let end = fragment.start.saturating_add(fragment.grapheme.len());
            self.string.drain(start..end);
            self.rebuild_fragments();
        }
    }

    // 删除行末尾的字符
    pub fn delete_last(&mut self) {
        self.delete(self.grapheme_count().saturating_sub(1));
    }

    // 将另一行的内容附加到当前行，并更新 fragments
    pub fn append(&mut self, other: &Self) {
        self.string.push_str(&other.string);
        self.rebuild_fragments();
    }

    // 在指定字素索引处拆分行，并返回拆分后的剩余部分
    pub fn split(&mut self, at: GraphemeIdx) -> Self {
        if let Some(fragment) = self.fragments.get(at) {
            let remainder = self.string.split_off(fragment.start);
            self.rebuild_fragments();
            Self::from(&remainder)
        } else {
            Self::default()
        }
    }

    // 将字节索引转换为字素索引
    fn byte_idx_to_grapheme_idx(&self, byte_idx: ByteIdx) -> Option<GraphemeIdx> {
        if byte_idx > self.string.len() {
            return None;
        }
        self.fragments
            .iter()
            .position(|fragment| fragment.start >= byte_idx)
    }

    // 将字素索引转换为字节索引
    fn grapheme_idx_to_byte_idx(&self, grapheme_idx: GraphemeIdx) -> ByteIdx {
        debug_assert!(grapheme_idx <= self.grapheme_count());
        if grapheme_idx == 0 || self.grapheme_count() == 0 {
            return 0;
        }
        self.fragments.get(grapheme_idx).map_or_else(
            || {
                #[cfg(debug_assertions)]
                {
                    panic!("Fragment not found for grapheme index: {grapheme_idx:?}");
                }
                #[cfg(not(debug_assertions))]
                {
                    0
                }
            },
            |fragment| fragment.start,
        )
    }

    // 从指定字素索引向前搜索查询字符串，并返回匹配的字素索引
    pub fn search_forward(
        &self,
        query: &str,
        from_grapheme_idx: GraphemeIdx,
    ) -> Option<GraphemeIdx> {
        debug_assert!(from_grapheme_idx <= self.grapheme_count());
        if from_grapheme_idx == self.grapheme_count() {
            return None;
        }
        let start = self.grapheme_idx_to_byte_idx(from_grapheme_idx);
        self.find_all(query, start..self.string.len())
            .first()
            .map(|(_, grapheme_idx)| *grapheme_idx)
    }

    // 从指定字素索引向后搜索查询字符串，并返回匹配的字素索引
    pub fn search_backward(
        &self,
        query: &str,
        from_grapheme_idx: GraphemeIdx,
    ) -> Option<GraphemeIdx> {
        debug_assert!(from_grapheme_idx <= self.grapheme_count());

        if from_grapheme_idx == 0 {
            return None;
        }
        let end_byte_index = if from_grapheme_idx == self.grapheme_count() {
            self.string.len()
        } else {
            self.grapheme_idx_to_byte_idx(from_grapheme_idx)
        };
        self.find_all(query, 0..end_byte_index)
            .last()
            .map(|(_, grapheme_idx)| *grapheme_idx)
    }

    // 在指定范围内查找查询字符串的所有匹配项，并返回匹配的字节索引和字素索引
    pub fn find_all(&self, query: &str, range: Range<ByteIdx>) -> Vec<(ByteIdx, GraphemeIdx)> {
        // Ensure that the range is valid and bounded by the string length
        let start = range.start;
        let end = min(range.end, self.string.len());
        debug_assert!(start <= end);
    
        // 根据给定的范围提取子字符串
        let substr = self.string.get(start..end);
    
        // 如果子字符串不可用，则提前返回
        if substr.is_none() {
            return Vec::new();
        }
    
        let substr = substr.unwrap();
        
        // 在子字符串中查找潜在匹配项
        let potential_matches: Vec<ByteIdx> = substr
            .match_indices(query)
            .map(|(relative_start_idx, _)| relative_start_idx.saturating_add(start))
            .collect();
    
        // 将潜在匹配项转换为与字素边界对齐的匹配项
        self.match_graphme_clusters(&potential_matches, query)
    }    
    
    // 查找与字素边界对齐的所有匹配项。
    // 参数：
    // - query：要搜索的查询。
    // - matches：潜在匹配项的字节索引向量，可能与字素簇对齐，也可能不对齐。
    // 返回：
    fn match_graphme_clusters(
        &self,
        matches: &[ByteIdx],
        query: &str,
    ) -> Vec<(ByteIdx, GraphemeIdx)> {
        let grapheme_count = query.graphemes(true).count();
        let query_graphemes: Vec<&str> = query.graphemes(true).collect();
    
        matches
            .iter()
            .filter_map(|&start| {
                self.byte_idx_to_grapheme_idx(start).and_then(|grapheme_idx| {
                    let end_idx = grapheme_idx.saturating_add(grapheme_count);
                    self.fragments
                        .get(grapheme_idx..end_idx)
                        .map(|fragments| {
                            let fragment_graphemes: Vec<&str> = fragments
                                .iter()
                                .map(|fragment| fragment.grapheme.as_str())
                                .collect();
                            (query_graphemes == fragment_graphemes).then_some((start, grapheme_idx))
                        })
                        .flatten() // 处理 Option<Option<(ByteIdx, GraphemeIdx)>> 类型
                })
            })
            .collect()
    }    
}

impl Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}

impl Deref for Line {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}