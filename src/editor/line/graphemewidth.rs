// 枚举 GraphemeWidth，表示字符的宽度，可以是 Half（半宽度）或 Full（全宽度）
#[derive(Copy, Clone, Debug)]
pub enum GraphemeWidth {
    Half,
    Full,
}
// 将 GraphemeWidth 转换为 usize 类型
impl From<GraphemeWidth> for usize {
    fn from(val: GraphemeWidth) -> Self {
        match val {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        }
    }
}
