use crate::prelude::*;
use std::io::Error;

pub trait UIComponent {
    // 标记此 UI 组件需要重绘（或不需要）
    fn set_needs_redraw(&mut self, value: bool);
    // 确定组件是否需要重绘
    fn needs_redraw(&self) -> bool;

    // 更新尺寸并标记为需要重绘
    fn resize(&mut self, size: Size) {
        self.set_size(size);
        self.set_needs_redraw(true);
    }
    // 更新尺寸。需要由每个组件实现。
    fn set_size(&mut self, size: Size);

    // 如果组件可见且需要重绘，则绘制此组件
    fn render(&mut self, origin_row: RowIdx) {
        if self.needs_redraw() {
            if let Err(err) = self.draw(origin_row) {
                #[cfg(debug_assertions)]
                {
                    panic!("无法渲染组件: {err:?}");
                }
                #[cfg(not(debug_assertions))]
                {
                    let _ = err;
                }
            } else {
                self.set_needs_redraw(false);
            }
        }
    }
    // 实际绘制组件的方法，必须由每个组件实现
    fn draw(&mut self, origin_row: RowIdx) -> Result<(), Error>;
}

