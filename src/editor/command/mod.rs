use crate::prelude::*;
use crossterm::event::Event;
use std::convert::TryFrom;
mod movecommand;
pub use movecommand::Move;
mod system;
pub use system::System;
mod edit;
pub use edit::Edit;

//  Command 枚举，用于表示不同类型的命令：移动命令、编辑命令和系统命令
#[derive(Clone, Copy)]
pub enum Command {
    Move(Move),
    Edit(Edit),
    System(System),
}

// clippy::as_conversions：在 usize < u16 的边缘情况下，会遇到问题
#[allow(clippy::as_conversions)]
impl TryFrom<Event> for Command {
    type Error = String;
    // 将 Event 转换为 Command
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(key_event) => Edit::try_from(key_event)
                .map(Command::Edit)
                .or_else(|_| Move::try_from(key_event).map(Command::Move))
                .or_else(|_| System::try_from(key_event).map(Command::System))
                .map_err(|_err| format!("Event not supported: {key_event:?}")),
            Event::Resize(width_u16, height_u16) => Ok(Self::System(System::Resize(Size {
                height: height_u16 as usize,
                width: width_u16 as usize,
            }))),
            _ => Err(format!("Event not supported: {event:?}")),
        }
    }
}
