use ratatui::{
    layout::Rect,
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// Wrap styled text to terminal cells without dropping long values or styles.
pub fn wrap(lines: Vec<Line<'static>>, width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut result = Vec::new();
    for line in lines {
        let indent = line
            .spans
            .first()
            .map(|span| {
                if line.spans.len() > 1 && span.content.starts_with("  ") {
                    span.content.width()
                } else {
                    span.content.chars().take_while(|ch| *ch == ' ').count()
                }
            })
            .unwrap_or(0)
            .min(width / 3);
        let mut spans = Vec::new();
        let mut used = 0;
        for span in line.spans {
            let mut part = String::new();
            for ch in span.content.chars() {
                let ch = if ch.is_control() && ch != '\n' {
                    ' '
                } else {
                    ch
                };
                let cells = ch.width().unwrap_or(0);
                if ch == '\n' || used + cells > width {
                    spans.push(Span::styled(std::mem::take(&mut part), span.style));
                    result.push(Line::from(std::mem::take(&mut spans)));
                    used = indent;
                    spans.push(Span::raw(" ".repeat(indent)));
                    if ch == '\n' {
                        continue;
                    }
                }
                // A wide glyph cannot fit a one-cell viewport.
                if cells > width {
                    part.push('…');
                    used += 1;
                } else {
                    part.push(ch);
                    used += cells;
                }
            }
            spans.push(Span::styled(part, span.style));
        }
        result.push(Line::from(spans));
    }
    result
}

/// The editor inserts at the end; keep that end and its cursor visible.
pub fn tail(text: &str, width: usize) -> String {
    let text = text.replace(['\n', '\r', '\t'], " ");
    if text.width() <= width {
        return text;
    }
    if width == 0 {
        return String::new();
    }
    let mut used = 0;
    let mut chars = Vec::new();
    for ch in text.chars().rev() {
        let cells = ch.width().unwrap_or(0);
        if used + cells > width.saturating_sub(1) {
            break;
        }
        used += cells;
        chars.push(ch);
    }
    format!("…{}", chars.into_iter().rev().collect::<String>())
}
