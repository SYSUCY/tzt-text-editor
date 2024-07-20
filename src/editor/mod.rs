use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use std::{
    env,
    io::Error,
    panic::{set_hook, take_hook},
};
use crate::prelude::*;

mod annotatedstring;
use annotatedstring::AnnotatedString;

mod command;
use command::{
    Command::{self, Edit, Move, System},
    Edit::InsertNewline,
    Move::{Down, Left, Right, Up},
    System::{Dismiss, Quit, Resize, Save, Search},
};

mod line;
use line::Line;

mod terminal;
use terminal::Terminal;

mod uicomponents;
use uicomponents::{View, CommandBar, MessageBar, StatusBar, UIComponent};

mod annotation;
use annotation::Annotation;

pub mod annotationtype;
pub use annotationtype::AnnotationType;

mod documentstatus;
use documentstatus::DocumentStatus;

mod filetype;
use filetype::FileType;

const QUIT_TIMES: u8 = 3;

#[derive(Eq, PartialEq, Default)]
enum PromptType {
    Search,
    Save,
    #[default]
    None,
}

impl PromptType {
    fn is_prompt(&self) -> bool {
        matches!(self, Self::Search | Self::Save)
    }
}

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    status_bar: StatusBar,
    message_bar: MessageBar,
    command_bar: CommandBar,
    prompt_type: PromptType,
    terminal_size: Size,
    title: String,
    quit_times: u8,
}

impl Editor {
    fn initialize_panic_hook() {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
    }

    // 初始化编辑器
    pub fn new() -> Result<Self, Error> {
        Self::initialize_panic_hook();
        // 初始化终端
        Terminal::initialize()?;

        let mut editor = Self::default();
        let size = Terminal::size().unwrap_or_default();
        editor.handle_resize_command(size);
        editor.update_message("帮助信息: Ctrl + F = 查找 | Ctrl + S = 保存 | Ctrl + Q = 退出");

        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            debug_assert!(!file_name.is_empty());
            if editor.view.load(file_name).is_err() {
                editor.update_message(&format!("ERROR: 无法打开文件: {file_name}"));
            }
        }
        editor.refresh_status();
        Ok(editor)
    }

    // 事件循环
    pub fn run(&mut self) {
        loop {
            self.refresh_screen();
            if self.should_quit {
                break;
            }
            match read() {
                Ok(event) => self.evaluate_event(event),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {err:?}");
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        // 错误提示
                        self.update_message("读取事件时发生错误，请重试。");
                    }
                }
            }
            self.refresh_status();
        }
    }

    fn refresh_screen(&mut self) {
        if self.terminal_size.height == 0 || self.terminal_size.width == 0 {
            return;
        }
        let bottom_bar_row = self.terminal_size.height.saturating_sub(1);
        let _ = Terminal::hide_caret();
        if self.in_prompt() {
            self.command_bar.render(bottom_bar_row);
        } else {
            self.message_bar.render(bottom_bar_row);
        }
        if self.terminal_size.height > 1 {
            self.status_bar
                .render(self.terminal_size.height.saturating_sub(2));
        }
        if self.terminal_size.height > 2 {
            self.view.render(0);
        }
        let new_caret_pos = if self.in_prompt() {
            Position {
                row: bottom_bar_row,
                col: self.command_bar.caret_position_col(),
            }
        } else {
            self.view.caret_position()
        };
        debug_assert!(new_caret_pos.col <= self.terminal_size.width);
        debug_assert!(new_caret_pos.row <= self.terminal_size.height);

        let _ = Terminal::move_caret_to(new_caret_pos);
        let _ = Terminal::show_caret();
        let _ = Terminal::execute();
    }

    fn refresh_status(&mut self) {
        let status = self.view.get_status();
        let title = format!("{} - {NAME}", status.file_name);
        self.status_bar.update_status(status);
        if title != self.title && matches!(Terminal::set_title(&title), Ok(())) {
            self.title = title;
        }
    }

    fn evaluate_event(&mut self, event: Event) {
        let should_process = match &event {
            Event::Key(KeyEvent { kind, .. }) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {
            if let Ok(command) = Command::try_from(event) {
                self.process_command(command);
            }
        }
    }

    //处理命令
    fn process_command(&mut self, command: Command) {
        match command {
            System(Resize(size)) => self.handle_resize_command(size),
            _ => match self.prompt_type {
                PromptType::Search => self.process_command_during_search(command),
                PromptType::Save => self.process_command_during_save(command),
                PromptType::None => self.process_command_no_prompt(command),
            }
        }
    }

    fn process_command_no_prompt(&mut self, command: Command) {
        if matches!(command, System(Quit)) {
            self.handle_quit_command();
            return;
        }
        self.reset_quit_times(); // 重置退出计数

        match command {
            System(Quit | Resize(_) | Dismiss) => {} // 退出和调整大小已经在上面处理，其他不适用
            System(Search) => self.set_prompt(PromptType::Search),
            System(Save) => self.handle_save_command(),
            Edit(edit_command) => self.view.handle_edit_command(edit_command),
            Move(move_command) => self.view.handle_move_command(move_command),
        }
    }

    // 处理调整大小命令
    fn handle_resize_command(&mut self, size: Size) {
        self.terminal_size = size;
        self.view.resize(Size {
            height: size.height.saturating_sub(2),
            width: size.width,
        });
        let bar_size = Size {
            height: 1,
            width: size.width,
        };
        self.message_bar.resize(bar_size);
        self.status_bar.resize(bar_size);
        self.command_bar.resize(bar_size);
    }

    // 处理退出命令
    fn handle_quit_command(&mut self) {
        if !self.view.get_status().is_modified || self.quit_times + 1 == QUIT_TIMES {
            self.should_quit = true;
        } else if self.view.get_status().is_modified {
            self.update_message(&format!(
                "WARNING! 文件有未保存的更改。再按 Ctrl-Q {} 次以退出。",
                QUIT_TIMES - self.quit_times - 1
            ));

            self.quit_times += 1;
        }
    }
    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.update_message("");
        }
    }
    
    // 处理保存模式下的命令
    fn handle_save_command(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        } else {
            self.set_prompt(PromptType::Save);
        }
    }

    fn process_command_during_save(&mut self, command: Command) {
        match command {
            System(Quit | Resize(_) | Search | Save) | Move(_) => {} // 保存过程中不适用，调整大小已经在此阶段处理
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.update_message("保存已取消。");
            }
            Edit(InsertNewline) => {
                let file_name = self.command_bar.value();
                self.save(Some(&file_name));
                self.set_prompt(PromptType::None);
            }
            Edit(edit_command) => self.command_bar.handle_edit_command(edit_command),
        }
    }
    
    fn save(&mut self, file_name: Option<&str>) {
        let result = if let Some(name) = file_name {
            self.view.save_as(name)
        } else {
            self.view.save()
        };
        if result.is_ok() {
            self.update_message("文件保存成功！");
        } else {
            self.update_message("文件写入失败！");
        }
    }

    // 处理查找模式下的命令
    fn process_command_during_search(&mut self, command: Command) {
        match command {
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.view.dismiss_search();
            }
            Edit(InsertNewline) => {
                self.set_prompt(PromptType::None);
                self.view.exit_search();
            }
            Edit(edit_command) => {
                self.command_bar.handle_edit_command(edit_command);
                let query = self.command_bar.value();
                self.view.search(&query);
            }
            Move(Right | Down) => self.view.search_next(),
            Move(Up | Left) => self.view.search_prev(),
            System(Quit | Resize(_) | Search | Save) | Move(_) => {} // 保存过程中不适用，调整大小已经在此阶段处理
        }
    }

    // 更新消息栏
    fn update_message(&mut self, new_message: &str) {
        self.message_bar.update_message(new_message);
    }

    // 判断是否在提示模式
    fn in_prompt(&self) -> bool {
        self.prompt_type.is_prompt()
    }

    // 设置提示模式
    fn set_prompt(&mut self, prompt_type: PromptType) {
        match prompt_type {
            PromptType::None => self.message_bar.set_needs_redraw(true), // 确保消息栏在下一个重绘周期中正确绘制
            PromptType::Save => self.command_bar.set_prompt("保存为（Esc 取消）: "),
            PromptType::Search => {
                self.view.enter_search();
                self.command_bar
                    .set_prompt("搜索（Esc 取消，箭头切换搜索结果）: ");
            }
        }
        self.command_bar.clear_value();
        self.prompt_type = prompt_type;
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("欢迎下次使用。\r\n");
        }
    }
}
