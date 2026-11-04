//! Headless-browser tests of the editor. The body of `web/index.html` and `web/style.css` are
//! embedded once and the editor started; tests then drive the page only through DOM events and
//! controls. Run them with `cargo test-editor` (see `.cargo/config.toml`).

use super::document;
use std::sync::Once;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{
    ClipboardEvent, ClipboardEventInit, DataTransfer, Element, HtmlElement, HtmlInputElement,
    HtmlSelectElement, HtmlTextAreaElement, KeyboardEvent, KeyboardEventInit, PointerEvent,
    PointerEventInit,
};

wasm_bindgen_test_configure!(run_in_browser);

const PAGE: &str = include_str!("../../web/index.html");
const STYLE: &str = include_str!("../../web/style.css");

fn element<T: JsCast>(id: &str) -> T {
    document()
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing #{id}"))
        .dyn_into()
        .unwrap_or_else(|_| panic!("#{id} has the wrong type"))
}
fn set_value(id: &str, value: &str) {
    element::<HtmlInputElement>(id).set_value(value);
}
fn click(id: &str) {
    let button = element::<HtmlElement>(id);
    assert!(!button.has_attribute("disabled"), "#{id} is disabled");
    button.click();
}
/// Click a checkbox only when it must change, so its change listener runs as for a user.
fn check(id: &str, on: bool) {
    let input = element::<HtmlInputElement>(id);
    if input.checked() != on {
        input.click();
    }
}
fn tool(name: &str) {
    let input: HtmlInputElement = document()
        .query_selector(&format!("input[name=tool][value={name}]"))
        .unwrap()
        .unwrap_or_else(|| panic!("missing tool {name}"))
        .dyn_into()
        .unwrap();
    input.click();
    assert!(input.checked());
}
fn status() -> String {
    element::<Element>("status")
        .text_content()
        .unwrap_or_default()
}
fn rows(lines: &[&str]) -> String {
    lines.join("\n")
}

/// The artwork text, checked against the ASCII-only contract on every read: printable bytes
/// 32..=126 in equal-width rows separated by LF.
fn art() -> String {
    let text = element::<Element>("art").text_content().unwrap_or_default();
    assert!(
        text.bytes().all(|b| b == b'\n' || (32..=126).contains(&b)),
        "#art holds a byte outside printable ASCII: {text:?}"
    );
    let widths: Vec<usize> = text.split('\n').map(str::len).collect();
    assert!(
        widths.iter().all(|w| *w == widths[0]),
        "#art rows differ in width: {widths:?}"
    );
    text
}
fn grid_size() -> (usize, usize) {
    let text = art();
    (
        text.split('\n').next().unwrap().len(),
        text.split('\n').count(),
    )
}

/// Load the page once, start the editor, then reset to a blank canvas with default tool
/// options. History and the area clipboard carry over between tests, as in one session.
fn blank(width: usize, height: usize) {
    static LOAD: Once = Once::new();
    LOAD.call_once(|| {
        let body = PAGE
            .split_once("<body>")
            .and_then(|(_, rest)| rest.rsplit_once("</body>"))
            .expect("index.html has a body")
            .0;
        let host = document().create_element("div").unwrap();
        // Scripts inserted as HTML do not run, so only `start` loads the editor.
        host.set_inner_html(&format!("<style>{STYLE}</style>{body}"));
        document().body().unwrap().append_child(&host).unwrap();
        super::start().expect("editor starts");
    });
    tool("select-text");
    check("diagram", false);
    check("arrow", true);
    check("onion-skin", false);
    check("mirror-glyphs", false);
    check("stamp-transparent", true);
    check("border", false);
    set_value("brush", "#");
    set_value("stamp", "");
    element::<HtmlSelectElement>("mode").set_value("blank");
    set_value("width", &width.to_string());
    set_value("height", &height.to_string());
    click("generate");
    assert_eq!(art(), rows(&vec![" ".repeat(width).as_str(); height]));
}

fn pointer(kind: &str, (x, y): (usize, usize)) {
    let (columns, lines) = grid_size();
    let rect = element::<Element>("art").get_bounding_client_rect();
    let init = PointerEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_pointer_id(1);
    init.set_pointer_type("mouse");
    init.set_is_primary(true);
    init.set_button(if kind == "pointermove" { -1 } else { 0 });
    // Aim at the centre of the cell.
    init.set_client_x(
        (rect.left() + (x as f64 + 0.5) * rect.width() / columns as f64).round() as i32,
    );
    init.set_client_y(
        (rect.top() + (y as f64 + 0.5) * rect.height() / lines as f64).round() as i32,
    );
    let event = PointerEvent::new_with_event_init_dict(kind, &init).unwrap();
    element::<Element>("art").dispatch_event(&event).unwrap();
}
/// Press at the first cell, move through the others and release at the last.
fn drag(path: &[(usize, usize)]) {
    pointer("pointerdown", path[0]);
    for point in &path[1..] {
        pointer("pointermove", *point);
    }
    pointer("pointerup", *path.last().unwrap());
}
fn key_with(key: &str, ctrl: bool, shift: bool) -> bool {
    let init = KeyboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(key);
    init.set_ctrl_key(ctrl);
    init.set_shift_key(shift);
    let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    let art = element::<HtmlElement>("art");
    art.focus().unwrap();
    art.dispatch_event(&event).unwrap();
    event.default_prevented()
}
fn keys(sequence: &[&str]) {
    for key in sequence {
        key_with(key, false, false);
    }
}
fn repeat(key: &str, times: usize) {
    for _ in 0..times {
        key_with(key, false, false);
    }
}
/// Move the keyboard cursor to a cell from the top-left corner.
fn cursor_to((x, y): (usize, usize)) {
    let (columns, lines) = grid_size();
    repeat("ArrowUp", lines);
    repeat("ArrowLeft", columns);
    repeat("ArrowDown", y);
    repeat("ArrowRight", x);
    assert_eq!(
        element::<Element>("cursor-position")
            .text_content()
            .unwrap(),
        format!("Column {}, row {}", x + 1, y + 1)
    );
}
/// Dispatch a clipboard event on the artwork and return the text it left on the clipboard.
fn clipboard(kind: &str, text: &str) -> String {
    let data = DataTransfer::new().unwrap();
    data.set_data("text/plain", text).unwrap();
    let init = ClipboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_clipboard_data(Some(&data));
    let event = ClipboardEvent::new_with_event_init_dict(kind, &init).unwrap();
    element::<Element>("art").dispatch_event(&event).unwrap();
    data.get_data("text/plain").unwrap()
}

#[wasm_bindgen_test]
fn pencil_draws_by_pointer_and_keyboard() {
    blank(8, 4);
    tool("pencil");
    drag(&[(1, 0), (4, 0), (4, 2)]);
    assert_eq!(
        art(),
        rows(&[" ####   ", "    #   ", "    #   ", "        "])
    );
    set_value("brush", "*");
    cursor_to((0, 3));
    keys(&[" ", "ArrowRight", "ArrowRight", " "]);
    assert_eq!(
        art(),
        rows(&[" ####   ", "    #   ", "    #   ", "***     "])
    );
}

#[wasm_bindgen_test]
fn saved_art_stamps_preview_cancel_clip_and_commit_as_one_undo_step() {
    blank(8, 4);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["A", " ", "B", "Enter", " ", "C", " "]);
    let original = rows(&["A B     ", " C      ", "        ", "        "]);
    assert_eq!(art(), original);
    tool("area");
    drag(&[(0, 0), (2, 1)]);
    click("save-stamp");
    let picker = element::<HtmlSelectElement>("art-stamp");
    let count = picker.child_element_count();
    click("save-stamp");
    assert_eq!(
        picker.child_element_count(),
        count,
        "same grid is not saved twice"
    );
    tool("stamp-art");
    pointer("pointerdown", (3, 1));
    assert_eq!(
        art(),
        rows(&["A B     ", " C A B  ", "    C   ", "        "])
    );
    pointer("pointermove", (5, 2));
    let placed = rows(&["A B     ", " C      ", "     A B", "      C "]);
    assert_eq!(art(), placed, "moving a preview leaves no trail");
    pointer("pointercancel", (5, 2));
    assert_eq!(art(), original);
    drag(&[(5, 2)]);
    assert_eq!(art(), placed);
    click("undo");
    assert_eq!(art(), original);
    click("redo");
    assert_eq!(art(), placed);
    // A stamp survives frame and project changes, but is not part of their history.
    blank(4, 2);
    tool("fill");
    drag(&[(0, 0)]);
    tool("stamp-art");
    cursor_to((1, 0));
    keys(&[" ", "Enter"]);
    assert_eq!(art(), rows(&["#A#B", "##C#"]));
    click("undo");
    check("stamp-transparent", false);
    cursor_to((1, 0));
    keys(&[" ", "Enter"]);
    assert_eq!(art(), rows(&["#A B", "# C "]));
    click("undo");
    cursor_to((3, 1));
    keys(&[" ", "Enter"]);
    assert_eq!(
        art(),
        rows(&["####", "###A"]),
        "edge placement clips safely"
    );
    click("undo");
    cursor_to((0, 0));
    keys(&[" ", "ArrowRight", "Escape"]);
    assert_eq!(art(), rows(&["####", "####"]));
    click("delete-stamp");
}

#[wasm_bindgen_test]
fn saved_art_stamp_library_is_bounded_and_removal_keeps_artwork() {
    blank(1, 1);
    // Each test starts with no saved entries, without inspecting application state.
    while !element::<Element>("delete-stamp").has_attribute("disabled") {
        click("delete-stamp");
    }
    for i in 0..16u8 {
        tool("type");
        cursor_to((0, 0));
        keys(&[&(char::from(b'A' + i)).to_string()]);
        tool("area");
        drag(&[(0, 0)]);
        click("save-stamp");
    }
    assert_eq!(element::<Element>("art-stamp").child_element_count(), 16);
    tool("type");
    cursor_to((0, 0));
    keys(&["Z"]);
    tool("area");
    drag(&[(0, 0)]);
    click("save-stamp");
    assert!(status().contains("limited to 16"));
    assert_eq!(element::<Element>("art-stamp").child_element_count(), 16);
    let picker = element::<HtmlSelectElement>("art-stamp");
    picker.set_value("0");
    picker
        .dispatch_event(&web_sys::Event::new("change").unwrap())
        .unwrap();
    tool("stamp-art");
    drag(&[(0, 0)]);
    assert_eq!(art(), "A", "choosing an earlier stamp uses that grid");
    click("undo");
    tool("area");
    drag(&[(0, 0)]);
    click("delete-stamp");
    assert_eq!(art(), "Z");
    click("save-stamp");
    assert_eq!(element::<Element>("art-stamp").child_element_count(), 16);
    while !element::<Element>("delete-stamp").has_attribute("disabled") {
        click("delete-stamp");
    }
    tool("stamp-art");
    drag(&[(0, 0)]);
    assert!(status().contains("save it as a stamp first"));
    assert_eq!(art(), "Z");
    assert_eq!(
        element::<Element>("art-stamp").text_content().unwrap(),
        "No stamps yet"
    );
}

#[wasm_bindgen_test]
fn saved_art_stamps_limit_total_cells_before_the_entry_limit() {
    blank(1, 1);
    while !element::<Element>("delete-stamp").has_attribute("disabled") {
        click("delete-stamp");
    }
    for i in 0..8u8 {
        blank(512, 256);
        tool("type");
        drag(&[(0, 0)]);
        keys(&[&char::from(b'A' + i).to_string()]);
        tool("area");
        drag(&[(0, 0), (511, 255)]);
        click("save-stamp");
        assert_eq!(
            element::<Element>("art-stamp").child_element_count(),
            usize::from((i + 1).min(7)) as u32
        );
    }
    assert!(status().contains("1,000,000 cells"));
    // A duplicate remains selectable even when a new full-size entry cannot fit.
    tool("type");
    drag(&[(0, 0)]);
    keys(&["A"]);
    tool("area");
    drag(&[(0, 0), (511, 255)]);
    click("save-stamp");
    assert!(status().contains("saved as a stamp"));
    assert_eq!(element::<HtmlSelectElement>("art-stamp").value(), "0");
    while !element::<Element>("delete-stamp").has_attribute("disabled") {
        click("delete-stamp");
    }
}

#[wasm_bindgen_test]
fn directional_mirror_is_optional_and_undoable() {
    blank(4, 2);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["/", "(", "<", "v", "Enter", "\\", ")", ">", "^"]);
    let original = rows(&["/(<v", "\\)>^"]);
    assert_eq!(art(), original);
    click("flip-horizontal");
    assert_eq!(art(), rows(&["v<(/", "^>)\\"]));
    click("undo");
    check("mirror-glyphs", true);
    click("flip-horizontal");
    assert_eq!(art(), rows(&["v>)\\", "^<(/"]));
    click("undo");
    assert_eq!(art(), original);
    click("flip-vertical");
    assert_eq!(art(), rows(&["/)>v", "\\(<^"]));
    click("undo");
    assert_eq!(art(), original);
}

#[wasm_bindgen_test]
fn eraser_clears_by_pointer_and_keyboard() {
    blank(6, 3);
    tool("fill");
    drag(&[(0, 0)]);
    assert_eq!(art(), rows(&["######"; 3]));
    tool("eraser");
    drag(&[(0, 0), (3, 0)]);
    cursor_to((5, 2));
    keys(&[" ", "ArrowLeft", "ArrowUp", " "]);
    assert_eq!(art(), rows(&["    ##", "#### #", "####  "]));
}

#[wasm_bindgen_test]
fn line_previews_from_its_start_and_draws_by_keyboard() {
    blank(7, 4);
    tool("line");
    set_value("brush", "-");
    // The preview redraws from the start each move, so the detour leaves no trace.
    pointer("pointerdown", (0, 0));
    pointer("pointermove", (3, 3));
    assert_eq!(art(), rows(&["-      ", " -     ", "  -    ", "   -   "]));
    pointer("pointermove", (6, 0));
    assert_eq!(art(), rows(&["-------", "       ", "       ", "       "]));
    pointer("pointerup", (6, 0));
    assert_eq!(art(), rows(&["-------", "       ", "       ", "       "]));
    set_value("brush", "|");
    cursor_to((3, 1));
    keys(&[" ", "ArrowDown", "ArrowDown"]);
    let drawn = rows(&["-------", "   |   ", "   |   ", "   |   "]);
    assert_eq!(art(), drawn);
    keys(&[" "]);
    assert_eq!(art(), drawn);
    // Escape abandons a keyboard line and its preview without changing the frame.
    keys(&[" ", "ArrowLeft"]);
    assert_eq!(art(), rows(&["-------", "   |   ", "   |   ", "  ||   "]));
    keys(&["Escape"]);
    assert_eq!(art(), drawn);
    click("undo");
    assert_eq!(art(), rows(&["-------", "       ", "       ", "       "]));
}

#[wasm_bindgen_test]
fn rectangle_draws_by_pointer_and_keyboard() {
    blank(9, 4);
    tool("rectangle");
    // Corners in either order give the same outline.
    drag(&[(3, 2), (0, 0)]);
    set_value("brush", "o");
    cursor_to((5, 1));
    keys(&[
        " ",
        "ArrowRight",
        "ArrowRight",
        "ArrowRight",
        "ArrowDown",
        "ArrowDown",
        " ",
    ]);
    assert_eq!(
        art(),
        rows(&["####     ", "#  # oooo", "#### o  o", "     oooo"])
    );
}

#[wasm_bindgen_test]
fn fill_replaces_the_touching_run_by_pointer_and_keyboard() {
    blank(7, 5);
    tool("rectangle");
    drag(&[(1, 1), (5, 3)]);
    tool("fill");
    set_value("brush", ".");
    drag(&[(3, 2)]);
    assert_eq!(
        art(),
        rows(&["       ", " ##### ", " #...# ", " ##### ", "       "])
    );
    set_value("brush", ":");
    cursor_to((0, 4));
    keys(&[" "]);
    assert_eq!(
        art(),
        rows(&[":::::::", ":#####:", ":#...#:", ":#####:", ":::::::"])
    );
}

#[wasm_bindgen_test]
fn type_writes_rows_by_pointer_keyboard_and_stamp() {
    blank(8, 3);
    tool("type");
    drag(&[(2, 0)]);
    keys(&["H", "i", "x", "Backspace", "Enter", "y", "o"]);
    assert_eq!(art(), rows(&["  Hi    ", "  yo    ", "        "]));
    // The keyboard alone chooses the cell; Stamp text needs no keyboard typing.
    cursor_to((5, 2));
    set_value("stamp", "ok!");
    click("stamp-text");
    assert_eq!(art(), rows(&["  Hi    ", "  yo    ", "     ok!"]));
}

#[wasm_bindgen_test]
fn area_selects_moves_copies_pastes_and_clears() {
    blank(10, 5);
    tool("type");
    drag(&[(1, 1)]);
    keys(&["A", "B", "Enter", "C", "D"]);
    tool("area");
    drag(&[(1, 1), (2, 2)]);
    assert!(!element::<Element>("selection").has_attribute("hidden"));
    assert_eq!(clipboard("copy", ""), "AB\nCD");
    // Dragging inside the selection moves its contents.
    drag(&[(1, 1), (5, 2)]);
    let moved = rows(&[
        "          ",
        "          ",
        "     AB   ",
        "     CD   ",
        "          ",
    ]);
    assert_eq!(art(), moved);
    // Pasting at the keyboard cursor selects the pasted area; Delete clears it.
    cursor_to((7, 0));
    clipboard("paste", "xy");
    assert_eq!(&art()[..10], "       xy ");
    keys(&["Delete"]);
    assert_eq!(art(), moved);
    // Keyboard selection and move, then cut, then paste and clear with the buttons.
    cursor_to((5, 2));
    keys(&[" ", "ArrowRight", "ArrowDown", " "]);
    // Space inside the selection lifts it; the arrow keys carry it.
    keys(&[" ", "ArrowLeft", "ArrowLeft", " "]);
    assert_eq!(
        art(),
        rows(&[
            "          ",
            "          ",
            "   AB     ",
            "   CD     ",
            "          ",
        ])
    );
    assert_eq!(clipboard("cut", ""), "AB\nCD");
    assert_eq!(art(), rows(&["          "; 5]));
    cursor_to((0, 0));
    click("area-paste");
    assert_eq!(&art()[..21], "AB        \nCD        ");
    click("area-clear");
    assert_eq!(art(), rows(&["          "; 5]));
    // The Copy area button fills the editor clipboard even where the system one is blocked.
    tool("type");
    drag(&[(0, 4)]);
    keys(&["z"]);
    tool("area");
    drag(&[(0, 4), (0, 4)]);
    click("area-copy");
    cursor_to((9, 0));
    click("area-paste");
    assert_eq!(
        art(),
        rows(&[
            "         z",
            "          ",
            "          ",
            "          ",
            "z         "
        ])
    );
}

#[wasm_bindgen_test]
fn diagram_boxes_and_connectors_join_with_plus() {
    blank(21, 7);
    check("diagram", true);
    tool("rectangle");
    set_value("stamp", "CLI");
    drag(&[(0, 0), (6, 2)]);
    set_value("stamp", "LIB");
    drag(&[(14, 0), (20, 2)]);
    // By keyboard, a box below the first.
    set_value("stamp", "OUT");
    cursor_to((0, 4));
    keys(&[" "]);
    repeat("ArrowRight", 6);
    keys(&["ArrowDown", "ArrowDown", " "]);
    tool("line");
    // Ending on the far box's edge keeps the edge; the arrowhead stops before it.
    drag(&[(6, 1), (14, 1)]);
    cursor_to((3, 2));
    keys(&[" ", "ArrowDown", "ArrowDown", " "]);
    assert_eq!(
        art(),
        rows(&[
            "+-----+       +-----+",
            "| CLI +------>| LIB |",
            "+--+--+       +-----+",
            "   v                 ",
            "+-----+              ",
            "| OUT |              ",
            "+-----+              ",
        ])
    );
}

#[wasm_bindgen_test]
fn onion_skin_stays_out_of_the_frame_and_selected_text() {
    blank(6, 2);
    tool("pencil");
    drag(&[(0, 0), (5, 0)]);
    click("add-frame");
    drag(&[(2, 1)]);
    check("onion-skin", true);
    let onion = element::<Element>("onion");
    assert!(!onion.has_attribute("hidden"));
    // The ghost shows only where the current frame is blank.
    assert_eq!(onion.text_content().unwrap(), rows(&["######", "      "]));
    assert_eq!(art(), rows(&["      ", "  #   "]));
    assert_eq!(
        element::<HtmlTextAreaElement>("editor").value(),
        rows(&["      ", "  #   "])
    );
    tool("select-text");
    let selection = web_sys::window().unwrap().get_selection().unwrap().unwrap();
    selection
        .select_all_children(&element::<Element>("grid"))
        .unwrap();
    let selected = String::from(js_sys::Object::to_string(&selection));
    selection.remove_all_ranges().unwrap();
    assert_eq!(selected.matches('#').count(), 1, "{selected:?}");
    // The first frame has no previous frame, so the ghost hides.
    click("previous");
    assert!(onion.has_attribute("hidden"));
    // Playback hides the ghost; pausing shows it again.
    click("next");
    assert!(!onion.has_attribute("hidden"));
    click("play");
    assert!(onion.has_attribute("hidden"));
    click("play");
    assert!(!onion.has_attribute("hidden"));
}

#[wasm_bindgen_test]
fn undo_and_redo_a_stroke_by_button_and_keyboard() {
    blank(5, 1);
    tool("pencil");
    drag(&[(0, 0), (2, 0)]);
    assert_eq!(art(), "###  ");
    click("undo");
    assert_eq!(art(), "     ");
    click("redo");
    assert_eq!(art(), "###  ");
    assert!(key_with("z", true, false));
    assert_eq!(art(), "     ");
    assert!(key_with("y", true, false));
    assert_eq!(art(), "###  ");
    key_with("z", true, false);
    assert!(key_with("Z", true, true));
    assert_eq!(art(), "###  ");
}

#[wasm_bindgen_test]
fn undo_reverts_a_typing_run_as_one_step() {
    blank(6, 1);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["a", "b", "c"]);
    // Moving the cursor ends the run, so this character is a step of its own.
    keys(&["ArrowRight", "d"]);
    assert_eq!(art(), "abc d ");
    key_with("z", true, false);
    assert_eq!(art(), "abc   ");
    key_with("z", true, false);
    assert_eq!(art(), "      ");
    key_with("y", true, false);
    assert_eq!(art(), "abc   ");
}

#[wasm_bindgen_test]
fn non_ascii_input_never_reaches_the_artwork() {
    blank(4, 2);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["é", "\u{1F600}", "\t", "a"]);
    assert_eq!(art(), rows(&["a   ", "    "]));
    // Choosing a tool replaces the status, so each error below is that step's own.
    tool("select-text");
    tool("type");
    clipboard("paste", "é");
    assert!(status().contains("printable ASCII"), "{}", status());
    tool("select-text");
    tool("type");
    set_value("stamp", "café");
    click("stamp-text");
    assert!(status().contains("printable ASCII"), "{}", status());
    assert_eq!(art(), rows(&["a   ", "    "]));
    tool("pencil");
    set_value("brush", "é");
    drag(&[(0, 1), (3, 1)]);
    assert!(status().contains("one printable ASCII"), "{}", status());
    set_value("brush", "\t");
    drag(&[(0, 1), (3, 1)]);
    assert_eq!(art(), rows(&["a   ", "    "]));
}

fn frame_count() -> String {
    element::<Element>("frame-count").text_content().unwrap()
}
fn input_event(id: &str, value: &str) {
    let field = element::<HtmlElement>(id);
    if let Some(input) = field.dyn_ref::<HtmlInputElement>() {
        input.set_value(value);
    } else {
        element::<HtmlTextAreaElement>(id).set_value(value);
    }
    field
        .dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}
/// Choose a file in the Open control as a user would, then wait for the read to finish.
async fn open(name: &str, contents: &str) -> String {
    let file =
        web_sys::File::new_with_str_sequence(&js_sys::Array::of1(&contents.into()), name).unwrap();
    let data = DataTransfer::new().unwrap();
    data.items().add_with_file(&file).unwrap();
    let input = element::<HtmlInputElement>("open-file");
    input.set_files(data.files().as_ref());
    input
        .dispatch_event(&web_sys::Event::new("change").unwrap())
        .unwrap();
    for _ in 0..500 {
        if !status().starts_with("Reading") {
            return status();
        }
        let tick = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10)
                .unwrap();
        });
        wasm_bindgen_futures::JsFuture::from(tick).await.unwrap();
    }
    panic!("{name} is still loading: {}", status());
}

#[wasm_bindgen_test]
async fn open_pads_text_restores_projects_and_refuses_unicode() {
    blank(3, 1);
    assert!(
        open("ragged.txt", "ab\r\ncd e")
            .await
            .contains("File opened")
    );
    assert_eq!(art(), rows(&["ab  ", "cd e"]));
    assert_eq!(element::<HtmlInputElement>("width").value(), "4");
    let project = crate::Animation::new(
        vec![
            crate::TimedFrame {
                frame: crate::Frame::from_text("ab").unwrap(),
                duration_ms: 70,
            },
            crate::TimedFrame {
                frame: crate::Frame::from_text("cd").unwrap(),
                duration_ms: 20,
            },
        ],
        false,
    )
    .unwrap();
    assert!(
        open("project.json", &project.to_json())
            .await
            .contains("File opened")
    );
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 1 / 2", "ab")
    );
    assert_eq!(element::<HtmlInputElement>("duration").value(), "70");
    assert!(!element::<HtmlInputElement>("loop").checked());
    click("next");
    assert_eq!(art(), "cd");
    assert_eq!(element::<HtmlInputElement>("duration").value(), "20");
    // A refused file leaves the current project in place.
    assert!(
        open("bad.txt", "caf\u{e9}")
            .await
            .contains("printable ASCII")
    );
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 2 / 2", "cd")
    );
}

#[wasm_bindgen_test]
fn frames_edit_reorder_flip_and_delete_from_the_timeline() {
    blank(3, 1);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["a"]);
    click("duplicate");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 2 / 2", "a  ")
    );
    // Unapplied edits block other actions until applied or discarded.
    input_event("editor", "b");
    click("previous");
    assert!(status().contains("Apply or discard"), "{}", status());
    assert_eq!(frame_count(), "Frame 2 / 2");
    input_event("duration", "120");
    click("apply");
    assert_eq!(art(), "b  ");
    input_event("timeline", "0");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 1 / 2", "a  ")
    );
    click("move-right");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 2 / 2", "a  ")
    );
    click("previous");
    assert_eq!(art(), "b  ");
    assert_eq!(element::<HtmlInputElement>("duration").value(), "120");
    click("flip-horizontal");
    assert_eq!(art(), "  b");
    input_event("editor", "zzz");
    click("discard");
    assert_eq!(art(), "  b");
    assert_eq!(element::<HtmlTextAreaElement>("editor").value(), "  b");
    click("delete-frame");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 1 / 1", "a  ")
    );
    click("undo");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 1 / 2", "  b")
    );
}

#[wasm_bindgen_test]
fn effect_animates_the_selected_frame_and_undoes() {
    blank(6, 1);
    tool("type");
    drag(&[(0, 0)]);
    keys(&["h", "i"]);
    element::<HtmlSelectElement>("effect").set_value("reveal");
    set_value("effect-duration", "500");
    set_value("effect-fps", "10");
    check("effect-reverse", false);
    click("apply-effect");
    assert!(
        status().contains("Effect applied: 5 frames"),
        "{}",
        status()
    );
    let mut frames = vec![art()];
    for _ in 1..5 {
        click("next");
        frames.push(art());
    }
    assert_eq!(frame_count(), "Frame 5 / 5");
    assert_eq!(frames.last().unwrap(), "hi    ");
    assert!(frames.iter().any(|frame| frame != "hi    "), "{frames:?}");
    click("undo");
    assert_eq!(
        (frame_count().as_str(), art().as_str()),
        ("Frame 1 / 1", "hi    ")
    );
}

#[wasm_bindgen_test]
fn artwork_search_filters_the_catalogue() {
    blank(4, 1);
    input_event("art-search", "bird");
    let matches = element::<Element>("art-matches").text_content().unwrap();
    assert!(
        matches.ends_with(" pieces") && !matches.starts_with("0 "),
        "{matches}"
    );
    let choice = element::<HtmlSelectElement>("art-name").value();
    assert!(!choice.is_empty());
    element::<HtmlSelectElement>("mode").set_value("art");
    click("generate");
    assert!(art().bytes().any(|b| b != b' ' && b != b'\n'));
    input_event("art-search", "no-such-piece");
    assert!(
        element::<Element>("art-matches")
            .text_content()
            .unwrap()
            .starts_with("No artwork matches")
    );
    input_event("art-search", "");
}
