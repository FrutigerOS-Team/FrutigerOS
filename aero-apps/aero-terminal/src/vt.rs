//! VT100/ANSI escape sequence parser.
//!
//! Phase 6: common CSI sequences (cursor move, basic colour, erase).
//! Phase 7: full VTE-compatible parser, consumed by the GPU text renderer.

/// A parsed terminal action.
#[derive(Debug, Clone, PartialEq)]
pub enum TermAction {
    Print(char),
    Newline,
    CarriageReturn,
    CursorMove { row: u16, col: u16 },
    SetColor { fg: u8, bg: u8 },
    ClearLine,
    ClearScreen,
    Bell,
    Unknown(Vec<u8>),
}

/// Incremental VT100 parser state.
#[derive(Debug, Default)]
pub struct ParserState {
    mode: ParseMode,
    /// Raw bytes of the escape sequence being collected.
    sequence: Vec<u8>,
    /// Current cursor position (0-based).
    row: u16,
    col: u16,
}

#[derive(Debug, Default, PartialEq)]
enum ParseMode {
    #[default]
    Normal,
    Escape,
    Csi,
}

/// Very simple VT100 parser — handles printable chars and common CSI escapes.
pub fn parse_byte(state: &mut ParserState, byte: u8) -> Option<TermAction> {
    match state.mode {
        ParseMode::Normal => match byte {
            0x07 => Some(TermAction::Bell),
            0x0A => {
                state.row = state.row.saturating_add(1);
                Some(TermAction::Newline)
            }
            0x0D => {
                state.col = 0;
                Some(TermAction::CarriageReturn)
            }
            0x1B => {
                state.mode = ParseMode::Escape;
                state.sequence.clear();
                None
            }
            b if b >= 0x20 && b != 0x7F => {
                state.col = state.col.saturating_add(1);
                Some(TermAction::Print(byte as char))
            }
            _ => None,
        },
        ParseMode::Escape => {
            if byte == b'[' {
                state.sequence.clear();
                state.mode = ParseMode::Csi;
                None
            } else {
                state.mode = ParseMode::Normal;
                let mut seq = std::mem::take(&mut state.sequence);
                seq.insert(0, 0x1B);
                seq.push(byte);
                Some(TermAction::Unknown(seq))
            }
        }
        ParseMode::Csi => {
            // CSI parameters are 0x30..=0x3F, intermediates 0x20..=0x2F,
            // and the final byte is 0x40..=0x7E.
            if (0x40..=0x7E).contains(&byte) {
                state.mode = ParseMode::Normal;
                let params = std::mem::take(&mut state.sequence);
                Some(dispatch_csi(state, &params, byte))
            } else if (0x20..=0x3F).contains(&byte) {
                state.sequence.push(byte);
                None
            } else {
                // Malformed sequence — emit it and recover.
                state.mode = ParseMode::Normal;
                let mut seq = std::mem::take(&mut state.sequence);
                seq.insert(0, 0x1B);
                seq.insert(1, b'[');
                seq.push(byte);
                Some(TermAction::Unknown(seq))
            }
        }
    }
}

/// Dispatch a complete CSI sequence. `params` excludes the leading `ESC [`.
fn dispatch_csi(state: &mut ParserState, params: &[u8], final_byte: u8) -> TermAction {
    let text = String::from_utf8_lossy(params);
    let nums: Vec<u16> = text
        .split(';')
        .map(|part| part.trim().parse::<u16>().unwrap_or(0))
        .collect();
    let first = nums.first().copied().unwrap_or(0);

    match final_byte {
        b'A' => {
            let n = first.max(1);
            state.row = state.row.saturating_sub(n);
            TermAction::CursorMove {
                row: state.row,
                col: state.col,
            }
        }
        b'B' => {
            let n = first.max(1);
            state.row = state.row.saturating_add(n);
            TermAction::CursorMove {
                row: state.row,
                col: state.col,
            }
        }
        b'C' => {
            let n = first.max(1);
            state.col = state.col.saturating_add(n);
            TermAction::CursorMove {
                row: state.row,
                col: state.col,
            }
        }
        b'D' => {
            let n = first.max(1);
            state.col = state.col.saturating_sub(n);
            TermAction::CursorMove {
                row: state.row,
                col: state.col,
            }
        }
        b'H' | b'f' => {
            // 1-based row;col, both default to 1.
            state.row = first.max(1).saturating_sub(1);
            state.col = nums.get(1).copied().unwrap_or(1).max(1).saturating_sub(1);
            TermAction::CursorMove {
                row: state.row,
                col: state.col,
            }
        }
        b'K' => TermAction::ClearLine,
        b'J' => TermAction::ClearScreen,
        b'm' => {
            // Map the common 8-colour SGR codes; keep prior colour otherwise.
            let mut fg = 7u8;
            let mut bg = 0u8;
            for code in &nums {
                match code {
                    0 => {
                        fg = 7;
                        bg = 0;
                    }
                    30..=37 => fg = (*code - 30) as u8,
                    40..=47 => bg = (*code - 40) as u8,
                    _ => {}
                }
            }
            TermAction::SetColor { fg, bg }
        }
        _ => {
            let mut seq = Vec::with_capacity(params.len() + 3);
            seq.push(0x1B);
            seq.push(b'[');
            seq.extend_from_slice(params);
            seq.push(final_byte);
            TermAction::Unknown(seq)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(state: &mut ParserState, bytes: &[u8]) -> Vec<TermAction> {
        bytes.iter().filter_map(|b| parse_byte(state, *b)).collect()
    }

    #[test]
    fn parse_printable() {
        let mut s = ParserState::default();
        assert_eq!(parse_byte(&mut s, b'A'), Some(TermAction::Print('A')));
        assert_eq!((s.row, s.col), (0, 1));
    }

    #[test]
    fn parse_newline() {
        let mut s = ParserState::default();
        assert_eq!(parse_byte(&mut s, 0x0A), Some(TermAction::Newline));
        assert_eq!((s.row, s.col), (1, 0));
    }

    #[test]
    fn escape_is_consumed() {
        let mut s = ParserState::default();
        // ESC followed by '[' — starts a CSI sequence.
        assert_eq!(parse_byte(&mut s, 0x1B), None);
        assert_eq!(parse_byte(&mut s, b'['), None);
    }

    #[test]
    fn cursor_up_and_down() {
        let mut s = ParserState::default();
        assert_eq!(
            feed(&mut s, b"\x1b[3B"),
            vec![TermAction::CursorMove { row: 3, col: 0 }]
        );
        assert_eq!(
            feed(&mut s, b"\x1b[2A"),
            vec![TermAction::CursorMove { row: 1, col: 0 }]
        );
    }

    #[test]
    fn cursor_position_is_zero_based() {
        let mut s = ParserState::default();
        assert_eq!(
            feed(&mut s, b"\x1b[5;10H"),
            vec![TermAction::CursorMove { row: 4, col: 9 }]
        );
    }

    #[test]
    fn erase_and_color() {
        let mut s = ParserState::default();
        assert_eq!(feed(&mut s, b"\x1b[K"), vec![TermAction::ClearLine]);
        assert_eq!(feed(&mut s, b"\x1b[2J"), vec![TermAction::ClearScreen]);
        assert_eq!(
            feed(&mut s, b"\x1b[31;44m"),
            vec![TermAction::SetColor { fg: 1, bg: 4 }]
        );
    }

    #[test]
    fn unknown_csi_is_reported() {
        let mut s = ParserState::default();
        assert_eq!(
            feed(&mut s, b"\x1b[?25h"),
            vec![TermAction::Unknown(b"\x1b[?25h".to_vec())]
        );
    }

    #[test]
    fn unknown_escape_is_reported() {
        let mut s = ParserState::default();
        assert_eq!(parse_byte(&mut s, 0x1B), None);
        assert_eq!(
            parse_byte(&mut s, b']'),
            Some(TermAction::Unknown(b"\x1b]".to_vec()))
        );
    }
}
