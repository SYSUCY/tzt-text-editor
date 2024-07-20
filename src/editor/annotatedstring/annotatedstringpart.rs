use super::AnnotationType;

// 结构体 AnnotatedStringPart，表示带注解字符串的一部分
#[derive(Debug)]
pub struct AnnotatedStringPart<'a> {
    pub string: &'a str,
    pub annotation_type: Option<AnnotationType>, // 注解类型，可选
}