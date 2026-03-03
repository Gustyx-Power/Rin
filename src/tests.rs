#[cfg(test)]
mod scrollback_tests {
    use crate::core::TerminalBuffer;

    #[test]
    fn test_scrollback_initially_empty() {
        let buffer = TerminalBuffer::new(80, 24);
        assert_eq!(buffer.scrollback_len(), 0);
        assert_eq!(buffer.scroll_offset(), 0);
    }

    #[test]
    fn test_scrollback_fills_on_scroll() {
        let mut buffer = TerminalBuffer::new(10, 3);

        for i in 0..5 {
            for c in format!("Line {}", i).chars() {
                buffer.write_char(c).unwrap();
            }
            buffer.write_char('\n').unwrap();
        }

        assert!(buffer.scrollback_len() > 0);
    }

    #[test]
    fn test_scroll_by() {
        let mut buffer = TerminalBuffer::new(80, 24);

        buffer.scroll_by(10);
        assert_eq!(buffer.scroll_offset(), 0);

        for _ in 0..50 {
            buffer.write_char('\n').unwrap();
        }

        buffer.scroll_by(5);
        assert!(buffer.scroll_offset() <= buffer.scrollback_len());
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut buffer = TerminalBuffer::new(80, 24);
        buffer.scroll_by(10);
        buffer.scroll_to_bottom();
        assert_eq!(buffer.scroll_offset(), 0);
    }
}

#[cfg(test)]
mod dirty_tracking_tests {
    use crate::core::{Cell, Grid};

    #[test]
    fn test_grid_initially_dirty() {
        let grid = Grid::new(80, 24);
        assert!(grid.has_dirty_rows());
        assert!(grid.is_row_dirty(0));
    }

    #[test]
    fn test_clear_dirty() {
        let mut grid = Grid::new(80, 24);
        grid.clear_dirty();
        assert!(!grid.has_dirty_rows());
        assert!(!grid.is_row_dirty(0));
    }

    #[test]
    fn test_set_marks_dirty() {
        let mut grid = Grid::new(80, 24);
        grid.clear_dirty();

        grid.set(5, 5, Cell::new('X')).unwrap();

        assert!(grid.is_row_dirty(5));
        assert!(!grid.is_row_dirty(0)); // Other rows still clean
    }

    #[test]
    fn test_get_mut_marks_dirty() {
        let mut grid = Grid::new(80, 24);
        grid.clear_dirty();

        if let Some(cell) = grid.get_mut(0, 10) {
            cell.character = 'Y';
        }

        assert!(grid.is_row_dirty(10));
    }
}

#[cfg(test)]
mod alternate_screen_tests {
    use crate::core::TerminalBuffer;

    #[test]
    fn test_not_alternate_initially() {
        let buffer = TerminalBuffer::new(80, 24);
        assert!(!buffer.is_alternate_screen());
    }

    #[test]
    fn test_enter_exit_alternate() {
        let mut buffer = TerminalBuffer::new(80, 24);

        // Write something first
        buffer.write_char('A').unwrap();
        let (x, y) = buffer.cursor_pos();

        // Enter alternate screen
        buffer.enter_alternate_screen();
        assert!(buffer.is_alternate_screen());
        assert_eq!(buffer.cursor_pos(), (0, 0)); // Cursor reset

        // Write in alternate
        buffer.write_char('B').unwrap();

        // Exit alternate screen
        buffer.exit_alternate_screen();
        assert!(!buffer.is_alternate_screen());
        assert_eq!(buffer.cursor_pos(), (x, y)); // Cursor restored
    }

    #[test]
    fn test_double_enter_noop() {
        let mut buffer = TerminalBuffer::new(80, 24);
        buffer.enter_alternate_screen();
        buffer.enter_alternate_screen(); // Should not crash or double-save
        assert!(buffer.is_alternate_screen());

        buffer.exit_alternate_screen();
        assert!(!buffer.is_alternate_screen());
    }
}

#[cfg(test)]
mod parser_tests {

    use crate::parser::{AnsiParser, Command};

    #[test]
    fn test_parse_256_color() {
        let mut parser = AnsiParser::new();

        // ESC[38;5;196m - 256-color foreground (red)
        let cmds = parser.parse(b"\x1b[38;5;196m").unwrap();

        let has_fg = cmds.iter().any(|c| matches!(c, Command::SetForeground(_)));
        assert!(has_fg, "Should parse 256-color foreground");
    }

    #[test]
    fn test_parse_true_color() {
        let mut parser = AnsiParser::new();

        // ESC[38;2;255;128;0m - RGB foreground (orange)
        let cmds = parser.parse(b"\x1b[38;2;255;128;0m").unwrap();

        let has_fg = cmds.iter().any(|c| {
            if let Command::SetForeground(color) = c {
                color.r == 255 && color.g == 128 && color.b == 0
            } else {
                false
            }
        });
        assert!(has_fg, "Should parse true color RGB");
    }

    #[test]
    fn test_parse_alternate_screen_enter() {
        let mut parser = AnsiParser::new();

        // ESC[?1049h - Enter alternate screen
        let cmds = parser.parse(b"\x1b[?1049h").unwrap();

        assert!(cmds.contains(&Command::EnterAlternateScreen));
    }

    #[test]
    fn test_parse_alternate_screen_exit() {
        let mut parser = AnsiParser::new();

        // ESC[?1049l - Exit alternate screen
        let cmds = parser.parse(b"\x1b[?1049l").unwrap();

        assert!(cmds.contains(&Command::ExitAlternateScreen));
    }

    #[test]
    fn test_parse_title() {
        let mut parser = AnsiParser::new();

        // OSC 0;Title BEL
        let cmds = parser.parse(b"\x1b]0;My Terminal\x07").unwrap();

        let has_title = cmds
            .iter()
            .any(|c| matches!(c, Command::SetTitle(t) if t == "My Terminal"));
        assert!(has_title, "Should parse OSC title");
    }

    #[test]
    fn test_parse_basic_colors() {
        let mut parser = AnsiParser::new();

        // ESC[31m - Red foreground
        let cmds = parser.parse(b"\x1b[31m").unwrap();

        let has_red = cmds
            .iter()
            .any(|c| matches!(c, Command::SetForeground(color) if color.r > 200));
        assert!(has_red, "Should parse basic red color");
    }
}

#[cfg(test)]
mod priority_feature_tests {
    use crate::parser::{AnsiParser, Charset, Command, CursorStyle};

    #[test]
    fn test_parse_decsc_decrc() {
        let mut parser = AnsiParser::new();

        let cmds = parser.parse(b"\x1b7").unwrap();
        assert!(
            cmds.contains(&Command::SaveCursor),
            "Should parse ESC 7 as SaveCursor"
        );
        let cmds = parser.parse(b"\x1b8").unwrap();
        assert!(
            cmds.contains(&Command::RestoreCursor),
            "Should parse ESC 8 as RestoreCursor"
        );
    }

    #[test]
    fn test_parse_cursor_style() {
        let mut parser = AnsiParser::new();

        let cmds = parser.parse(b"\x1b[2 q").unwrap();
        assert!(
            cmds.contains(&Command::SetCursorStyle(CursorStyle::SteadyBlock)),
            "Should parse CSI 2 SP q as SteadyBlock"
        );

        let cmds = parser.parse(b"\x1b[5 q").unwrap();
        assert!(
            cmds.contains(&Command::SetCursorStyle(CursorStyle::BlinkBar)),
            "Should parse CSI 5 SP q as BlinkBar"
        );
    }

    #[test]
    fn test_parse_bracketed_paste() {
        let mut parser = AnsiParser::new();

        let cmds = parser.parse(b"\x1b[?2004h").unwrap();
        assert!(
            cmds.contains(&Command::SetBracketedPaste(true)),
            "Should parse ?2004h as enable bracketed paste"
        );

        let cmds = parser.parse(b"\x1b[?2004l").unwrap();
        assert!(
            cmds.contains(&Command::SetBracketedPaste(false)),
            "Should parse ?2004l as disable bracketed paste"
        );
    }

    #[test]
    fn test_parse_line_drawing_charset() {
        let mut parser = AnsiParser::new();

        let cmds = parser.parse(b"\x1b(0").unwrap();
        assert!(
            cmds.contains(&Command::SetCharset(Charset::LineDrawing)),
            "Should parse ESC (0 as LineDrawing charset"
        );

        let cmds = parser.parse(b"\x1b(B").unwrap();
        assert!(
            cmds.contains(&Command::SetCharset(Charset::Ascii)),
            "Should parse ESC (B as Ascii charset"
        );
    }

    #[test]
    fn test_parse_tab_stop_commands() {
        let mut parser = AnsiParser::new();

        let cmds = parser.parse(b"\x1bH").unwrap();
        assert!(
            cmds.contains(&Command::SetTabStop),
            "Should parse ESC H as SetTabStop"
        );

        let cmds = parser.parse(b"\x1b[0g").unwrap();
        assert!(
            cmds.contains(&Command::ClearTabStop),
            "Should parse CSI 0g as ClearTabStop"
        );

        let cmds = parser.parse(b"\x1b[3g").unwrap();
        assert!(
            cmds.contains(&Command::ClearAllTabStops),
            "Should parse CSI 3g as ClearAllTabStops"
        );
    }
}

#[cfg(test)]
mod scroll_region_tests {
    use crate::core::TerminalBuffer;
    use crate::parser::Command;

    fn put_char(buf: &mut TerminalBuffer, x: usize, y: usize, c: char) {
        buf.execute_command(Command::MoveCursor(x, y)).unwrap();
        buf.write_char(c).unwrap();
    }

    fn get_char(buf: &TerminalBuffer, x: usize, y: usize) -> char {
        buf.grid().get(x, y).map(|c| c.character).unwrap_or(' ')
    }

    #[test]
    fn test_scroll_region_scroll_up() {
        let mut buf = TerminalBuffer::new(10, 8);

        // Put identifiable chars in rows 2-5
        put_char(&mut buf, 0, 2, 'A');
        put_char(&mut buf, 0, 3, 'B');
        put_char(&mut buf, 0, 4, 'C');
        put_char(&mut buf, 0, 5, 'D');
        // Put a char outside the region
        put_char(&mut buf, 0, 0, 'Z');
        put_char(&mut buf, 0, 7, 'Y');

        // Set scroll region to rows 2-5 (1-indexed: 3-6)
        buf.execute_command(Command::SetScrollRegion { top: 2, bottom: 5 })
            .unwrap();
        buf.execute_command(Command::ScrollUp(1)).unwrap();

        // Row 0 and 7 should be untouched
        assert_eq!(get_char(&buf, 0, 0), 'Z');
        assert_eq!(get_char(&buf, 0, 7), 'Y');
        assert_eq!(get_char(&buf, 0, 2), 'B');
        assert_eq!(get_char(&buf, 0, 3), 'C');
        assert_eq!(get_char(&buf, 0, 4), 'D');
        assert_eq!(get_char(&buf, 0, 5), ' ');
    }

    #[test]
    fn test_scroll_region_scroll_down() {
        let mut buf = TerminalBuffer::new(10, 8);

        put_char(&mut buf, 0, 2, 'A');
        put_char(&mut buf, 0, 3, 'B');
        put_char(&mut buf, 0, 4, 'C');
        put_char(&mut buf, 0, 5, 'D');
        put_char(&mut buf, 0, 0, 'Z');

        buf.execute_command(Command::SetScrollRegion { top: 2, bottom: 5 })
            .unwrap();
        buf.execute_command(Command::ScrollDown(1)).unwrap();

        assert_eq!(get_char(&buf, 0, 0), 'Z');
        assert_eq!(get_char(&buf, 0, 2), ' ');
        assert_eq!(get_char(&buf, 0, 3), 'A');
        assert_eq!(get_char(&buf, 0, 4), 'B');
        assert_eq!(get_char(&buf, 0, 5), 'C');
    }

    #[test]
    fn test_insert_line_at_cursor() {
        let mut buf = TerminalBuffer::new(10, 8);

        put_char(&mut buf, 0, 1, 'A');
        put_char(&mut buf, 0, 2, 'B');
        put_char(&mut buf, 0, 3, 'C');
        put_char(&mut buf, 0, 5, 'E');

        buf.execute_command(Command::SetScrollRegion { top: 1, bottom: 5 })
            .unwrap();
        // Move cursor to row 2  
        buf.execute_command(Command::MoveCursor(0, 2)).unwrap();
        buf.execute_command(Command::InsertLine(1)).unwrap();

        // Row 1 untouched
        assert_eq!(get_char(&buf, 0, 1), 'A');
        // Row 2 should be blank
        assert_eq!(get_char(&buf, 0, 2), ' ');
        // B shifted to row 3, C to row 4
        assert_eq!(get_char(&buf, 0, 3), 'B');
        assert_eq!(get_char(&buf, 0, 4), 'C');
        // E was at row 5 but row 4 shifts to row 5, overwriting E
        assert_eq!(get_char(&buf, 0, 5), ' ');
    }

    #[test]
    fn test_delete_line_at_cursor() {
        let mut buf = TerminalBuffer::new(10, 8);

        put_char(&mut buf, 0, 1, 'A');
        put_char(&mut buf, 0, 2, 'B');
        put_char(&mut buf, 0, 3, 'C');
        put_char(&mut buf, 0, 4, 'D');

        buf.execute_command(Command::SetScrollRegion { top: 1, bottom: 4 })
            .unwrap();
        buf.execute_command(Command::MoveCursor(0, 2)).unwrap();
        buf.execute_command(Command::DeleteLine(1)).unwrap();

        // Row 1 untouched
        assert_eq!(get_char(&buf, 0, 1), 'A');
        assert_eq!(get_char(&buf, 0, 2), 'C');
        assert_eq!(get_char(&buf, 0, 3), 'D');
        assert_eq!(get_char(&buf, 0, 4), ' ');
    }

    #[test]
    fn test_newline_at_scroll_region_bottom() {
        let mut buf = TerminalBuffer::new(10, 8);

        put_char(&mut buf, 0, 2, 'A');
        put_char(&mut buf, 0, 3, 'B');
        put_char(&mut buf, 0, 4, 'C');
        // Outside region
        put_char(&mut buf, 0, 0, 'Z');
        put_char(&mut buf, 0, 7, 'Y');

        buf.execute_command(Command::SetScrollRegion { top: 2, bottom: 4 })
            .unwrap();
        buf.execute_command(Command::MoveCursor(0, 4)).unwrap();
        buf.execute_command(Command::Print('\n')).unwrap();

        // Outside region untouched
        assert_eq!(get_char(&buf, 0, 0), 'Z');
        assert_eq!(get_char(&buf, 0, 7), 'Y');
        assert_eq!(get_char(&buf, 0, 2), 'B');
        assert_eq!(get_char(&buf, 0, 3), 'C');
        assert_eq!(get_char(&buf, 0, 4), ' ');
    }
}
