use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

#[cfg(feature = "markdown")]
use crate::markdown::parse_inline_formatting;
use crate::theme::RichTextTheme;

pub fn render_edit_mode(
    text: &str,
    cursor_char_idx: usize,
    horizontal_scroll: usize,
    max_width: usize,
    password: bool,
    placeholder: Option<&str>,
    theme: &impl RichTextTheme,
) -> Vec<Line<'static>> {
    if text.is_empty() {
        let display = placeholder.unwrap_or("");
        return vec![Line::from(vec![Span::styled(
            display.to_string(),
            Style::default().fg(theme.get_muted_text_color()),
        )])];
    }

    if password {
        let masked = "*".repeat(text.chars().count());
        return vec![Line::from(Span::styled(
            masked,
            Style::default().fg(theme.get_text_color()),
        ))];
    }

    let cursor_line_idx = char_offset_to_line(text, cursor_char_idx);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (line_idx, raw_line) in text.split('\n').enumerate() {
        let spans = style_source_spans(raw_line, theme);
        let line = Line::from(spans);
        if line_idx == cursor_line_idx {
            lines.push(apply_horizontal_scroll(&line, horizontal_scroll, max_width));
        } else {
            lines.push(line);
        }
    }

    lines
}

pub(in crate::text_input) fn char_offset_to_line(text: &str, char_idx: usize) -> usize {
    let mut offset = 0usize;
    for (i, line) in text.split('\n').enumerate() {
        let line_len = line.chars().count();
        if offset + line_len >= char_idx {
            return i;
        }
        offset += line_len + 1;
    }
    text.split('\n').count().saturating_sub(1)
}

pub(in crate::text_input) fn char_offset_to_line_col(
    text: &str,
    char_idx: usize,
) -> (usize, usize) {
    let mut offset = 0usize;
    for (i, line) in text.split('\n').enumerate() {
        let line_len = line.chars().count();
        if offset + line_len >= char_idx {
            return (i, char_idx - offset);
        }
        offset += line_len + 1;
    }
    let last_line_len = text
        .split('\n')
        .next_back()
        .map(|l| l.chars().count())
        .unwrap_or(0);
    let num_lines = text.split('\n').count().saturating_sub(1);
    (num_lines, last_line_len)
}

pub(in crate::text_input) fn expanded_display_col(raw_line: &str, raw_col: usize) -> usize {
    let mut display_col = 0usize;
    for (i, ch) in raw_line.chars().enumerate() {
        if i >= raw_col {
            break;
        }
        if ch == '\t' {
            display_col += 4;
        } else {
            display_col += 1;
        }
    }
    display_col
}

pub(in crate::text_input) fn line_col_to_char_offset(
    text: &str,
    line_idx: usize,
    col: usize,
) -> usize {
    let mut offset = 0usize;
    for (i, line) in text.split('\n').enumerate() {
        if i == line_idx {
            return offset + col.min(line.chars().count());
        }
        offset += line.chars().count() + 1;
    }
    text.chars().count()
}

fn style_source_spans(text: &str, theme: &impl RichTextTheme) -> Vec<Span<'static>> {
    let expanded = text.replace('\t', "    ");
    #[cfg(feature = "markdown")]
    if let Some(spans) = style_block_marker(&expanded, theme) {
        return spans;
    }
    let mut spans = Vec::new();
    for segment in classify_edit_segments(&expanded) {
        match segment {
            EditSegment::Plain(value) => {
                if !value.is_empty() {
                    spans.push(Span::styled(
                        value,
                        Style::default().fg(theme.get_text_color()),
                    ));
                }
            }
            EditSegment::Delimited { open, content, close } => {
                if open == "[" && close == ")" {
                    push_link_spans(&mut spans, &content, theme);
                } else {
                    let content_style = inline_content_style(&content, theme);
                    let marker = Style::default().fg(theme.get_muted_text_color());
                    if !open.is_empty() {
                        spans.push(Span::styled(open, marker));
                    }
                    if !content.is_empty() {
                        spans.push(Span::styled(content, content_style));
                    }
                    if !close.is_empty() {
                        spans.push(Span::styled(close, marker));
                    }
                }
            }
        }
    }
    spans
}

fn push_link_spans(spans: &mut Vec<Span<'static>>, content: &str, theme: &impl RichTextTheme) {
    let Some((label, url)) = content.split_once("](") else {
        spans.push(Span::styled(
            content.to_string(),
            Style::default().fg(theme.get_text_color()),
        ));
        return;
    };
    let marker = Style::default().fg(theme.get_muted_text_color());
    spans.push(Span::styled("[".to_string(), marker));
    spans.push(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme.get_primary_color())
            .add_modifier(Modifier::UNDERLINED),
    ));
    spans.push(Span::styled("](".to_string(), marker));
    spans.push(Span::styled(url.to_string(), marker));
    spans.push(Span::styled(")".to_string(), marker));
}

#[cfg(feature = "markdown")]
fn inline_content_style(content: &str, theme: &impl RichTextTheme) -> Style {
    parse_inline_formatting(content, theme)
        .into_iter()
        .find(|span| !span.content.is_empty())
        .map(|span| span.style)
        .unwrap_or_else(|| Style::default().fg(theme.get_text_color()))
}

#[cfg(not(feature = "markdown"))]
fn inline_content_style(_content: &str, theme: &impl RichTextTheme) -> Style {
    Style::default().fg(theme.get_text_color())
}

#[cfg(feature = "markdown")]
fn style_block_marker(text: &str, theme: &impl RichTextTheme) -> Option<Vec<Span<'static>>> {
    let bytes = text.as_bytes();
    let mut hashes = 0usize;
    while hashes < bytes.len() && hashes < 6 && bytes[hashes] == b'#' {
        hashes += 1;
    }
    if hashes > 0 && bytes.get(hashes) == Some(&b' ') {
        let marker_style = Style::default()
            .fg(theme.get_primary_color())
            .add_modifier(Modifier::BOLD);
        let rest_style = Style::default()
            .fg(theme.get_text_color())
            .add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(text[..hashes].to_string(), marker_style)];
        if hashes < text.len() {
            spans.push(Span::styled(text[hashes..].to_string(), rest_style));
        }
        return Some(spans);
    }
    if matches!(bytes.first(), Some(b'-' | b'*' | b'+')) && bytes.get(1) == Some(&b' ') {
        let marker = Style::default().fg(theme.get_muted_text_color());
        return Some(vec![
            Span::styled(text[..1].to_string(), marker),
            Span::styled(text[1..].to_string(), Style::default().fg(theme.get_text_color())),
        ]);
    }
    None
}

enum EditSegment {
    Plain(String),
    Delimited {
        open: String,
        content: String,
        close: String,
    },
}

fn classify_edit_segments(text: &str) -> Vec<EditSegment> {
    let chars: Vec<char> = text.chars().collect();
    let mut segments = Vec::new();
    let mut plain = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if let Some((end, open_len, close_len)) = match_delimited(&chars, i) {
            if !plain.is_empty() {
                segments.push(EditSegment::Plain(std::mem::take(&mut plain)));
            }
            segments.push(EditSegment::Delimited {
                open: chars[i..i + open_len].iter().collect(),
                content: chars[i + open_len..end - close_len].iter().collect(),
                close: chars[end - close_len..end].iter().collect(),
            });
            i = end;
            continue;
        }
        plain.push(chars[i]);
        i += 1;
    }
    if !plain.is_empty() {
        segments.push(EditSegment::Plain(plain));
    }
    segments
}

fn match_delimited(chars: &[char], i: usize) -> Option<(usize, usize, usize)> {
    if chars[i] == '`' {
        let run = tick_run(chars, i);
        return match_run(chars, i, run).map(|end| (end, run, run));
    }
    if chars[i] == '~' && chars.get(i + 1) == Some(&'~') {
        return find_closer(chars, i + 2, &['~', '~']).map(|end| (end, 2, 2));
    }
    if chars[i] == '*' || chars[i] == '_' {
        let run = if chars.get(i + 1) == Some(&chars[i]) && chars.get(i + 2) == Some(&chars[i]) {
            3
        } else if chars.get(i + 1) == Some(&chars[i]) {
            2
        } else if is_left_flanking(chars, i) {
            1
        } else {
            return None;
        };
        let marker = vec![chars[i]; run];
        return find_closer(chars, i + run, &marker).map(|end| (end, run, run));
    }
    if chars[i] == '[' {
        let label_end = find_unescaped(chars, i + 1, ']')?;
        if chars.get(label_end + 1) != Some(&'(') {
            return None;
        }
        let url_end = find_unescaped(chars, label_end + 2, ')')?;
        return Some((url_end + 1, 1, 1));
    }
    None
}

fn match_run(chars: &[char], i: usize, run: usize) -> Option<usize> {
    if run == 0 {
        return None;
    }
    let marker = chars[i];
    let mut j = i + run;
    while j < chars.len() {
        if chars[j] == marker && tick_run(chars, j) == run {
            return Some(j + run);
        }
        j += 1;
    }
    None
}

fn tick_run(chars: &[char], i: usize) -> usize {
    let Some(marker) = chars.get(i).copied() else {
        return 0;
    };
    let mut n = 0usize;
    while chars.get(i + n) == Some(&marker) {
        n += 1;
    }
    n
}

fn find_closer(chars: &[char], from: usize, marker: &[char]) -> Option<usize> {
    if marker.is_empty() {
        return None;
    }
    let mut j = from;
    while j + marker.len() <= chars.len() {
        if chars[j..j + marker.len()] == *marker && (j == 0 || chars[j - 1] != '\\') {
            return Some(j + marker.len());
        }
        j += 1;
    }
    None
}

fn find_unescaped(chars: &[char], from: usize, target: char) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j] == target && (j == 0 || chars[j - 1] != '\\') {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn is_left_flanking(chars: &[char], i: usize) -> bool {
    i == 0 || matches!(chars[i - 1], ' ' | '\t' | '\n' | '(' | '[')
}

fn apply_horizontal_scroll(line: &Line<'_>, scroll: usize, max_width: usize) -> Line<'static> {
    let mut skip = scroll;
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut collected = 0usize;

    for span in &line.spans {
        let span_text: String = span.content.clone().into();
        let w = unicode_width::UnicodeWidthStr::width(span_text.as_str());
        if skip > 0 {
            if w <= skip {
                skip -= w;
                continue;
            }
            let chars: Vec<char> = span_text.chars().collect();
            let mut ci = 0;
            while ci < chars.len() && skip > 0 {
                let cw = unicode_width::UnicodeWidthChar::width(chars[ci]).unwrap_or(0);
                skip = skip.saturating_sub(cw);
                ci += 1;
            }
            let remaining: String = chars[ci..].iter().collect();
            let rw = unicode_width::UnicodeWidthStr::width(remaining.as_str());
            if collected + rw <= max_width {
                result.push(Span::styled(remaining, span.style));
                collected += rw;
            } else {
                let trunc = truncate_to_width(&remaining, max_width - collected);
                let tw = unicode_width::UnicodeWidthStr::width(trunc.as_str());
                result.push(Span::styled(trunc, span.style));
                collected += tw;
            }
        } else if collected + w <= max_width {
            result.push(Span::styled(span_text, span.style));
            collected += w;
        } else {
            let trunc = truncate_to_width(&span_text, max_width - collected);
            let tw = unicode_width::UnicodeWidthStr::width(trunc.as_str());
            result.push(Span::styled(trunc, span.style));
            collected += tw;
        }

        if collected >= max_width {
            break;
        }
    }

    Line::from(result)
}

fn truncate_to_width(s: &str, max_w: usize) -> String {
    let mut result = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max_w {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeConfig;

    fn theme() -> ThemeConfig {
        ThemeConfig::default()
    }

    #[test]
    fn empty_text_shows_nothing() {
        let lines = render_edit_mode("", 0, 0, 80, false, None, &theme());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn plain_text_renders() {
        let lines = render_edit_mode("hello world", 0, 0, 80, false, None, &theme());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn multiline_text_renders_multiple_lines() {
        let lines = render_edit_mode("hello\nworld", 5, 0, 80, false, None, &theme());
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn password_mode_masks_text() {
        let lines = render_edit_mode("secret", 0, 0, 80, true, None, &theme());
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        let content: String = line.spans.iter().map(|s| s.content.clone()).collect();
        assert_eq!(content, "******");
    }

    #[test]
    fn placeholder_shown_when_empty() {
        let lines = render_edit_mode("", 0, 0, 80, false, Some("type here"), &theme());
        let content: String = lines[0].spans.iter().map(|s| s.content.clone()).collect();
        assert_eq!(content, "type here");
    }

    #[test]
    fn bold_delimiters_styled() {
        let spans = style_source_spans("**bold**", &theme());
        assert!(spans.len() >= 3);
    }

    #[test]
    fn italic_delimiters_styled() {
        let spans = style_source_spans("*italic*", &theme());
        assert!(spans.len() >= 3);
    }

    #[test]
    fn inline_code_styled() {
        let spans = style_source_spans("`code`", &theme());
        assert!(spans.len() >= 3);
    }

    #[test]
    fn link_styled() {
        let spans = style_source_spans("[text](url)", &theme());
        assert!(spans.len() >= 5);
    }

    #[test]
    fn tab_expanded_to_spaces_in_style_source_spans() {
        let spans = style_source_spans("\thello", &theme());
        let content: String = spans.iter().map(|s| s.content.clone()).collect();
        assert!(
            !content.contains('\t'),
            "tab should be expanded to spaces: {:?}",
            content
        );
        assert!(
            content.starts_with("    "),
            "tab should expand to 4 spaces: {:?}",
            content
        );
    }

    #[test]
    fn expanded_display_col_with_tabs() {
        assert_eq!(expanded_display_col("abc", 3), 3);
        assert_eq!(expanded_display_col("\tbc", 1), 4);
        assert_eq!(expanded_display_col("\t\tc", 2), 8);
        assert_eq!(expanded_display_col("a\tc", 2), 5);
        assert_eq!(expanded_display_col("", 0), 0);
    }
}
