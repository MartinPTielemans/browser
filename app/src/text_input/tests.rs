//! A field as an input method asks it where its caret is.

use gpui::{Bounds, TestAppContext, VisualTestContext};

use super::*;

/// A window showing a field alone, focused.
struct Shows(Entity<TextInput>);

impl Render for Shows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.clone())
    }
}

fn shown(text: &str, cx: &mut TestAppContext) -> (Entity<TextInput>, VisualTestContext) {
    cx.update(bind_keys);
    let (shows, cx) = cx.add_window_view(|window, cx| {
        let input = cx.new(|cx| TextInput::new("Search or Enter URL…", cx));
        window.focus(&input.focus_handle(cx), cx);
        Shows(input)
    });
    let input = cx.read(|cx| shows.read(cx).0.clone());
    if !text.is_empty() {
        cx.simulate_input(text);
    }
    cx.run_until_parked();
    (input, cx.clone())
}

fn caret(
    input: &Entity<TextInput>,
    at: usize,
    cx: &mut VisualTestContext,
) -> Option<Bounds<Pixels>> {
    let field = Bounds::new(point(px(10.), px(20.)), size(px(200.), px(30.)));
    input.update_in(cx, |input, window, cx| {
        input.bounds_for_range(at..at, field, window, cx)
    })
}

// Under Wayland the browser's input method composes for a field only once it knows where its
// caret is, and the command bar opens empty, showing its placeholder.
#[gpui::test]
fn an_empty_field_has_its_caret_at_its_start(cx: &mut TestAppContext) {
    let (input, mut cx) = shown("", cx);
    let caret = caret(&input, 0, &mut cx).expect("no caret");
    assert_eq!(
        (caret.origin, caret.size.width),
        (point(px(10.), px(20.)), px(0.))
    );
}

#[gpui::test]
fn a_field_has_its_caret_after_what_it_holds(cx: &mut TestAppContext) {
    let (input, mut cx) = shown("abc", cx);
    let caret = caret(&input, 3, &mut cx).expect("no caret");
    assert!(
        caret.origin.x > px(10.),
        "the caret is at {:?}",
        caret.origin
    );
}
