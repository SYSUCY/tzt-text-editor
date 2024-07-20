use super::{Annotation, AnnotationType, Line, SyntaxHighlighter};
use crate::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

const KEYWORDS: [&str; 52] = [
    "break",
    "const",
    "continue",
    "crate",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "async",
    "await",
    "dyn",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "try",
    "macro_rules",
    "union",
];
const TYPES: [&str; 22] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "Option", "Result", "String", "str", "Vec", "HashMap",
];

const KNOWN_VALUES: [&str; 6] = ["Some", "None", "true", "false", "Ok", "Err"];

#[derive(Default)]
pub struct RustSyntaxHighlighter {
    highlights: Vec<Vec<Annotation>>,
    ml_comment_balance: usize,
    in_ml_string: bool,
}
impl RustSyntaxHighlighter {
    fn annotate_ml_comment(&mut self, string: &str) -> Option<Annotation> {
        let mut chars = string.char_indices().peekable();

        while let Some((_, char)) = chars.next() {
            if char == '/' {
                //检查多行注释的起始符号
                if let Some((_, '*')) = chars.peek() {
                    self.ml_comment_balance = self.ml_comment_balance.saturating_add(1);
                    chars.next();
                }
            } else if self.ml_comment_balance == 0 {
                return None; // 没有找到起始符号，且当前不在多行注释中，返回None
            } else if char == '*' {
                if let Some((idx, '/')) = chars.peek() {
                    self.ml_comment_balance = self.ml_comment_balance.saturating_sub(1);

                    if self.ml_comment_balance == 0 {
                        return Some(Annotation {
                            annotation_type: AnnotationType::Comment,
                            start: 0,
                            end: idx.saturating_add(1),
                        });
                    }
                    chars.next();
                }
            }
        }

        (self.ml_comment_balance > 0).then_some(Annotation {
            annotation_type: AnnotationType::Comment,
            start: 0,
            end: string.len(),
        })
    }
    fn annotate_string(&mut self, string: &str) -> Option<Annotation> {
        let mut chars = string.char_indices();
        while let Some((idx, char)) = chars.next() {
            if char == '\\' && self.in_ml_string {
                chars.next(); // 跳过转义字符
                continue;
            }
            if char == '"' {
                if self.in_ml_string {
                    self.in_ml_string = false;
                    return Some(Annotation {
                        annotation_type: AnnotationType::String,
                        start: 0,
                        end: idx.saturating_add(1),
                    });
                }
                self.in_ml_string = true;
            }
            if !self.in_ml_string {
                return None;
            }
        }
        self.in_ml_string.then_some(Annotation {
            annotation_type: AnnotationType::String,
            start: 0,
            end: string.len(),
        })
    }
    fn initial_annotation(&mut self, line: &Line) -> Option<Annotation> {
        if self.in_ml_string {
            self.annotate_string(line)
        } else if self.ml_comment_balance > 0 {
            self.annotate_ml_comment(line)
        } else {
            None
        }
    }

    fn annotate_remainder(&mut self, remainder: &str) -> Option<Annotation> {
        self.annotate_ml_comment(remainder)
            .or_else(|| self.annotate_string(remainder))
            .or_else(|| annotate_single_line_comment(remainder))
            .or_else(|| annotate_char(remainder))
            .or_else(|| annotate_lifetime_specifier(remainder))
            .or_else(|| annotate_number(remainder))
            .or_else(|| annotate_keyword(remainder))
            .or_else(|| annotate_type(remainder))
            .or_else(|| annotate_known_value(remainder))
    }
}
impl SyntaxHighlighter for RustSyntaxHighlighter {
    fn highlight(&mut self, idx: LineIdx, line: &Line) {
        debug_assert_eq!(idx, self.highlights.len());
        let mut result = Vec::new();
        let mut iterator = line.split_word_bound_indices().peekable();
        if let Some(annotation) = self.initial_annotation(line) {
            //处理悬挂的多行注释或字符串

            result.push(annotation);
            // 跳过已经注释过的单词
            while let Some(&(next_idx, _)) = iterator.peek() {
                if next_idx >= annotation.end {
                    break;
                }
                iterator.next();
            }
        }
        while let Some((start_idx, _)) = iterator.next() {
            let remainder = &line[start_idx..];
            if let Some(mut annotation) = self.annotate_remainder(remainder) {
                annotation.shift(start_idx);
                result.push(annotation);
                // 跳过已经注释过的单词
                while let Some(&(next_idx, _)) = iterator.peek() {
                    if next_idx >= annotation.end {
                        break;
                    }
                    iterator.next();
                }
            };
        }
        self.highlights.push(result);
    }

    fn get_annotations(&self, idx: LineIdx) -> Option<&Vec<Annotation>> {
        self.highlights.get(idx)
    }
}

fn annotate_next_word<F>(
    string: &str,
    annotation_type: AnnotationType,
    validator: F,
) -> Option<Annotation>
where
    F: Fn(&str) -> bool,
{
    if let Some(word) = string.split_word_bounds().next() {
        if validator(word) {
            return Some(Annotation {
                annotation_type,
                start: 0,
                end: word.len(),
            });
        }
    }
    None
}

fn annotate_number(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Number, is_valid_number)
}

fn annotate_type(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Type, is_type)
}

fn annotate_keyword(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::Keyword, is_keyword)
}

fn annotate_known_value(string: &str) -> Option<Annotation> {
    annotate_next_word(string, AnnotationType::KnownValue, is_known_value)
}

fn annotate_char(string: &str) -> Option<Annotation> {
    let mut iter = string.split_word_bound_indices().peekable();
    if let Some((_, "\'")) = iter.next() {
        if let Some((_, "\\")) = iter.peek() {
            iter.next(); //跳过转义字符
        }
        iter.next(); //跳到结束引号
        if let Some((idx, "\'")) = iter.next() {
            return Some(Annotation {
                annotation_type: AnnotationType::Char,
                start: 0,
                end: idx.saturating_add(1), //包含结束引号
            });
        }
    }
    None
}

fn annotate_lifetime_specifier(string: &str) -> Option<Annotation> {
    let mut iter = string.split_word_bound_indices();
    if let Some((_, "\'")) = iter.next() {
        if let Some((idx, next_word)) = iter.next() {
            return Some(Annotation {
                annotation_type: AnnotationType::LifetimeSpecifier,
                start: 0,
                end: idx.saturating_add(next_word.len()),
            });
        }
    }
    None
}

fn annotate_single_line_comment(string: &str) -> Option<Annotation> {
    if string.starts_with("//") {
        return Some(Annotation {
            annotation_type: AnnotationType::Comment,
            start: 0,
            end: string.len(),
        });
    }
    None
}

fn is_valid_number(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    if is_numeric_literal(word) {
        return true;
    }
    let mut chars = word.chars();

    // 检查第一个字符
    if let Some(first_char) = chars.next() {
        if !first_char.is_ascii_digit() {
            return false; // 数字必须以数字开头
        }
    }

    let mut seen_dot = false;
    let mut seen_e = false;
    let mut prev_was_digit = true;
    // 迭代剩余字符
    for char in chars {
        match char {
            '0'..='9' => {
                prev_was_digit = true;
            }
            '_' => {
                if !prev_was_digit {
                    return false; // 下划线必须在数字之间
                }
                prev_was_digit = false;
            }
            '.' => {
                if seen_dot || seen_e || !prev_was_digit {
                    return false; // 禁止多个点，禁止点在'e'之后，禁止点不在数字之后
                }
                seen_dot = true;
                prev_was_digit = false;
            }
            'e' | 'E' => {
                if seen_e || !prev_was_digit {
                    return false; // 禁止多个'e'或'e'不在数字之后
                }
                seen_e = true;
                prev_was_digit = false;
            }
            _ => {
                return false; // 非法字符
            }
        }
    }

    prev_was_digit // 必须以数字结束
}

fn is_numeric_literal(word: &str) -> bool {
    if word.len() < 3 {
        //对于字面量，我们需要一个前导'0'，一个后缀和至少一个数字
        return false;
    }
    let mut chars = word.chars();
    if chars.next() != Some('0') {
        // 检查第一个字符是否为前导0
        return false;
    }
    let base = match chars.next() {
        // 检查第二个字符是否为有效基数
        Some('b' | 'B') => 2,
        Some('o' | 'O') => 8,
        Some('x' | 'X') => 16,
        _ => return false,
    };
    chars.all(|char| char.is_digit(base))
}
fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}
fn is_type(word: &str) -> bool {
    TYPES.contains(&word)
}

fn is_known_value(word: &str) -> bool {
    KNOWN_VALUES.contains(&word)
}
