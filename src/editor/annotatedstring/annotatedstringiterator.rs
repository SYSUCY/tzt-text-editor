use crate::prelude::*;
use std::cmp::min;

use super::{AnnotatedString, AnnotatedStringPart};

// 结构体 AnnotatedStringIterator 用于迭代 AnnotatedString
pub struct AnnotatedStringIterator<'a> {
    pub annotated_string: &'a AnnotatedString, // 注释字符串
    pub current_idx: ByteIdx, // 当前字节序号
}

impl<'a> Iterator for AnnotatedStringIterator<'a> {
    type Item = AnnotatedStringPart<'a>;
    // 返回迭代器的下一个元素
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.annotated_string.string.len() {
            return None;
        }

        let annotations = &self.annotated_string.annotations;
        let current_idx = self.current_idx;

        // 查找当前活动注释（如果有）
        if let Some(annotation) = annotations.iter().find(|annotation| {
            annotation.start <= current_idx && annotation.end > current_idx
        }) {
            let end_idx = min(annotation.end, self.annotated_string.string.len());
            let start_idx = self.current_idx;
            self.current_idx = end_idx;

            return Some(AnnotatedStringPart {
                string: &self.annotated_string.string[start_idx..end_idx],
                annotation_type: Some(annotation.annotation_type),
            });
        }

        // 查找最近的注释边界（如果有）
        let end_idx = annotations.iter()
            .filter(|annotation| annotation.start > current_idx)
            .map(|annotation| annotation.start)
            .min()
            .unwrap_or(self.annotated_string.string.len());

        let start_idx = self.current_idx;
        self.current_idx = end_idx;

        Some(AnnotatedStringPart {
            string: &self.annotated_string.string[start_idx..end_idx],
            annotation_type: None,
        })
    }
}