mod editor;
use editor::Editor;
mod prelude;

fn main() {
    Editor::new().unwrap().run();
}
