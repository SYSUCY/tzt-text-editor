use crate::prelude::*;

use super::GraphemeWidth;
// 结构体，包含 grapheme（字素字符串）、rendered_width（渲染宽度）、replacement（替代字符）、start（开始位置）
#[derive(Clone, Debug)]
pub struct TextFragment {
    pub grapheme: String,
    pub rendered_width: GraphemeWidth,
    pub replacement: Option<char>,
    pub start: ByteIdx,
}
