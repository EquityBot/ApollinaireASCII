// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

use apollinaire_ascii::{Canvas, Frame, canvas::bounds};

fn canvas(text: &str) -> Canvas {
    Canvas::from_frame(&Frame::from_text(text).unwrap())
}
fn blank(width: usize, height: usize) -> Canvas {
    Canvas::new(width, height).unwrap()
}
fn text(canvas: Canvas) -> String {
    canvas.finish().text()
}

#[test]
fn bounds_normalise_corners_in_any_order() {
    assert_eq!(bounds((4, 1), (1, 3)), (1, 1, 4, 3));
    assert_eq!(bounds((2, 2), (2, 2)), (2, 2, 1, 1));
    assert_eq!(
        bounds((i32::MIN, 0), (i32::MAX, 0)).2,
        u32::MAX as usize + 1
    );
}

#[test]
fn get_reads_cells_and_reports_outside_as_none() {
    let grid = canvas("ab\ncd");
    assert_eq!(grid.get(1, 1), Some(b'd'));
    assert_eq!(
        (grid.get(-1, 0), grid.get(2, 0), grid.get(0, 2)),
        (None, None, None)
    );
}

#[test]
fn ellipse_outlines_are_symmetrical_at_odd_and_even_sizes() {
    let mut grid = blank(9, 5);
    grid.ellipse(0, 0, 9, 5, b'#').unwrap();
    assert_eq!(
        text(grid),
        "  #####  \n##     ##\n#       #\n##     ##\n  #####  "
    );
    let mut grid = blank(6, 4);
    grid.ellipse(0, 0, 6, 4, b'o').unwrap();
    assert_eq!(text(grid), " oooo \no    o\no    o\n oooo ");
}

#[test]
fn ellipse_collapses_to_a_line_or_cell_clips_and_validates() {
    let mut grid = blank(5, 3);
    grid.ellipse(1, 1, 3, 1, b'-').unwrap();
    grid.ellipse(0, 0, 1, 3, b'|').unwrap();
    grid.ellipse(4, 2, 1, 1, b'.').unwrap();
    assert_eq!(text(grid), "|    \n|--- \n|   .");
    let mut grid = blank(4, 3);
    grid.ellipse(-2, -1, 5, 3, b'#').unwrap();
    assert_eq!(text(grid), "  # \n##  \n    ");
    let mut grid = blank(2, 2);
    grid.ellipse(i32::MAX, i32::MIN, 3, 3, b'#').unwrap();
    assert_eq!(text(grid), "  \n  ");
    assert!(blank(2, 2).ellipse(0, 0, 0, 1, b'#').is_err());
    assert!(blank(2, 2).ellipse(0, 0, 1, 1, b'\n').is_err());
}

#[test]
fn fill_area_is_solid_clipped_and_validated() {
    let mut grid = blank(4, 3);
    grid.fill_area(-1, 1, 3, 9, b'#').unwrap();
    assert_eq!(text(grid), "    \n##  \n##  ");
    let mut grid = blank(2, 2);
    grid.fill_area(i32::MAX, i32::MIN, usize::MAX, usize::MAX, b'x')
        .unwrap();
    assert_eq!(text(grid), "  \n  ");
    let mut grid = blank(2, 2);
    grid.fill_area(0, 0, usize::MAX, usize::MAX, b'x').unwrap();
    assert_eq!(text(grid), "xx\nxx");
    assert!(blank(2, 2).fill_area(0, 0, 1, 1, b'\n').is_err());
}

#[test]
fn flood_fill_replaces_only_the_four_connected_run() {
    let mut grid = canvas("#####\n#   #\n#####\n     ");
    grid.flood_fill(2, 1, b'.').unwrap();
    assert_eq!(text(grid), "#####\n#...#\n#####\n     ");
    // Corners touch the start cell only diagonally, so they stay blank.
    let mut grid = canvas(" # \n# #\n # ");
    grid.flood_fill(0, 0, b'.').unwrap();
    assert_eq!(text(grid), ".# \n# #\n # ");
    let mut grid = canvas("ab\nab");
    grid.flood_fill(0, 0, b'a').unwrap();
    grid.flood_fill(5, 5, b'z').unwrap();
    assert_eq!(text(grid), "ab\nab");
    assert!(blank(1, 1).flood_fill(0, 0, 0x7f).is_err());
}

#[test]
fn flood_fill_covers_the_largest_grid() {
    let mut grid = blank(apollinaire_ascii::MAX_WIDTH, apollinaire_ascii::MAX_HEIGHT);
    grid.flood_fill(100, 100, b'~').unwrap();
    assert!(grid.finish().cells().iter().all(|cell| *cell == b'~'));
}

#[test]
fn diagram_boxes_have_corners_labels_and_join_crossings() {
    let mut grid = blank(13, 5);
    grid.diagram_box((10, 4), (0, 0), "VARROCK").unwrap();
    assert_eq!(
        text(grid),
        "+---------+  \n|         |  \n| VARROCK |  \n|         |  \n+---------+  "
    );
    let mut grid = blank(7, 5);
    grid.diagram_box((0, 0), (4, 2), "").unwrap();
    grid.diagram_box((2, 1), (6, 4), "").unwrap();
    assert_eq!(text(grid), "+---+  \n| +-+-+\n+-+-+ |\n  |   |\n  +---+");
    let mut grid = blank(5, 3);
    grid.diagram_box((0, 0), (4, 2), "LUMBRIDGE").unwrap();
    assert_eq!(text(grid), "+---+\n|LUM|\n+---+");
}

#[test]
fn connectors_bend_once_join_boxes_and_point_their_arrows() {
    let mut grid = blank(8, 4);
    grid.connector((0, 0), (6, 3), true).unwrap();
    assert_eq!(text(grid), "------+ \n      | \n      | \n      v ");
    let mut grid = blank(3, 5);
    grid.connector((0, 4), (2, 0), true).unwrap();
    assert_eq!(text(grid), "+->\n|  \n|  \n|  \n|  ");
    for (end, head) in [
        ((0, 2), b'<'),
        ((2, 0), b'^'),
        ((4, 2), b'>'),
        ((2, 4), b'v'),
    ] {
        let mut grid = blank(5, 5);
        grid.connector((2, 2), end, true).unwrap();
        assert_eq!(grid.get(end.0, end.1), Some(head));
    }
    let mut grid = blank(3, 3);
    grid.connector((1, 1), (1, 1), true).unwrap();
    assert_eq!(text(grid), "   \n + \n   ");
}

#[test]
fn connectors_cross_lines_and_attach_to_box_edges() {
    let mut grid = blank(7, 5);
    grid.connector((3, 0), (3, 4), false).unwrap();
    grid.connector((0, 2), (6, 2), true).unwrap();
    assert_eq!(text(grid), "   |   \n   |   \n---+-->\n   |   \n   |   ");
    let mut grid = blank(8, 3);
    grid.diagram_box((0, 0), (3, 2), "").unwrap();
    grid.connector((3, 1), (7, 1), true).unwrap();
    assert_eq!(text(grid), "+--+    \n|  +--->\n+--+    ");
}

#[test]
fn arrowheads_keep_the_edges_they_reach_and_join_lines_that_cross_them() {
    let mut grid = blank(8, 3);
    grid.diagram_box((4, 0), (7, 2), "").unwrap();
    grid.connector((0, 1), (4, 1), true).unwrap();
    assert_eq!(text(grid), "    +--+\n--->|  |\n    +--+");
    let mut grid = blank(8, 3);
    grid.connector((3, 0), (3, 2), false).unwrap();
    grid.diagram_box((4, 0), (7, 2), "").unwrap();
    grid.connector((0, 1), (4, 1), true).unwrap();
    assert_eq!(text(grid), "   |+--+\n-->+|  |\n   |+--+");
    // Every cell before the edge is a junction, so no arrowhead fits.
    let mut grid = blank(5, 3);
    for x in 1..4 {
        grid.connector((x, 0), (x, 2), false).unwrap();
    }
    grid.diagram_box((4, 0), (4, 2), "").unwrap();
    grid.connector((1, 1), (4, 1), true).unwrap();
    assert_eq!(text(grid), " |||+\n +++|\n |||+");
    let mut grid = blank(5, 3);
    grid.connector((2, 0), (2, 2), true).unwrap();
    grid.connector((0, 2), (4, 2), false).unwrap();
    assert_eq!(text(grid), "  |  \n  |  \n--+--");
}

#[test]
fn diagram_output_is_plain_ascii_and_input_is_validated() {
    let mut grid = blank(4, 4);
    assert!(grid.diagram_box((0, 0), (3, 3), "caf\u{e9}").is_err());
    assert!(grid.diagram_box((0, 0), (5000, 3), "").is_err());
    assert!(grid.connector((0, 0), (0, -5000), false).is_err());
    grid.diagram_box((-3, -3), (9, 9), "").unwrap();
    grid.connector((-2, 1), (9, 1), true).unwrap();
    let frame = grid.finish();
    assert!(frame.cells().iter().all(|b| b" +-|<>^v".contains(b)));
}
