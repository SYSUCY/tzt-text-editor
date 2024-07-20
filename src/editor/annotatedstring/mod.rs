use std::{
    cmp::{max, min},
    fmt::{self, Display},
};
use crate::editor::{Annotation, AnnotationType, ByteIdx};

mod annotatedstringiterator;
use annotatedstringiterator::AnnotatedStringIterator;

mod annotatedstringpart;
use annotatedstringpart::AnnotatedStringPart;

#[derive(Default, Debug)]
pub struct AnnotatedString {
    string: String,
    annotations: Vec<Annotation>, // 包含注释的数组
}

impl AnnotatedString {
    // 从字符串创建一个 AnnotatedString
    pub fn from(string: &str) -> Self {
        Self {
            string: String::from(string),
            annotations: Vec::new(),
        }
    }

    // 添加注解
    pub fn add_annotation(
        &mut self,
        annotation_type: AnnotationType,
        start: ByteIdx,
        end: ByteIdx,
    ) {
        debug_assert!(start <= end);
        self.annotations.push(Annotation {
            annotation_type,
            start,
            end,
        });
    }

    // 从左侧截断字符串直到指定索引
    pub fn truncate_left_until(&mut self, until: ByteIdx) {
        self.replace(0, until, "");
    }

    // 从指定索引开始向右截断字符串
    pub fn truncate_right_from(&mut self, from: ByteIdx) {
        self.replace(from, self.string.len(), "");
    }
    
    // 替换字符串的某个范围，并调整注解
    pub fn replace(&mut self, start: ByteIdx, end: ByteIdx, new_string: &str) {
        let end = min(end, self.string.len());
        debug_assert!(start <= end);
        debug_assert!(start <= self.string.len());
        if start > end {
            return;
        }
        self.string.replace_range(start..end, new_string);
    
        let replaced_range_len = end - start;
        let len_difference = new_string.len().abs_diff(replaced_range_len);
    
        if len_difference == 0 {
            return;
        }
    
        let adjust_annotation = |idx: &mut ByteIdx, _boundary: ByteIdx, shortened: bool| {
            if *idx >= end {
                if shortened {
                    *idx = idx.saturating_sub(len_difference);
                } else {
                    *idx += len_difference;
                }
            } else if *idx > start {
                if shortened {
                    *idx = max(start, idx.saturating_sub(len_difference));
                } else {
                    *idx = min(end, idx.saturating_add(len_difference));
                }
            }
        };
    
        self.annotations.iter_mut().for_each(|annotation| {
            adjust_annotation(&mut annotation.start, start, new_string.len() < replaced_range_len);
            adjust_annotation(&mut annotation.end, start, new_string.len() < replaced_range_len);
        });
    
        self.annotations.retain(|annotation| annotation.start < annotation.end && annotation.start < self.string.len());
    }    
}

impl Display for AnnotatedString {
    // 实现 fmt 方法，用于格式化输出
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}
 
impl<'a> IntoIterator for &'a AnnotatedString {
    type Item = AnnotatedStringPart<'a>; // 迭代器的元素类型是 AnnotatedStringPart
    type IntoIter = AnnotatedStringIterator<'a>; // 迭代器类型是 AnnotatedStringIterator

    // 返回一个 AnnotatedStringIterator
    fn into_iter(self) -> Self::IntoIter {
        AnnotatedStringIterator {
            annotated_string: self,
            current_idx: 0,
        }
    }
}