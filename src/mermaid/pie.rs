use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::types::PieChart;
use crate::theme::RichTextTheme;

const BLOCK: &str = "█";
const LIGHT_BLOCK: &str = "░";

pub fn parse_pie(source: &str) -> Option<PieChart> {
    let mut title: Option<String> = None;
    let mut slices: Vec<(String, f64)> = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("pie") {
            let rest = line
                .strip_prefix("pie")
                .expect("already checked with starts_with")
                .trim();
            if let Some(t) = rest.strip_prefix("title ") {
                title = Some(t.trim().to_string());
            }
            continue;
        }

        if line.starts_with("title ") {
            title = Some(
                line.strip_prefix("title ")
                    .expect("already checked with starts_with")
                    .trim()
                    .to_string(),
            );
            continue;
        }

        if let Some(slice) = parse_slice(line) {
            slices.push(slice);
        }
    }

    if slices.is_empty() {
        return None;
    }

    Some(PieChart { title, slices })
}

fn parse_slice(line: &str) -> Option<(String, f64)> {
    let parsed = super::parser::parse_rule(super::parser::Rule::pie_slice, line)?;
    let label = super::parser::text_of(&parsed, super::parser::Rule::pie_label)?;
    let value_part = super::parser::text_of(&parsed, super::parser::Rule::pie_value)?;
    let value: f64 = value_part.parse().ok()?;
    if value <= 0.0 || label.is_empty() {
        return None;
    }
    Some((label, value))
}

pub fn render_pie(
    diagram: &PieChart,
    max_width: usize,
    theme: &impl RichTextTheme,
) -> Vec<Line<'static>> {
    if diagram.slices.is_empty() {
        return vec![Line::from(Span::styled(
            "(empty pie chart)",
            Style::default().fg(theme.get_muted_text_color()),
        ))];
    }

    let title_style = Style::default()
        .fg(theme.get_primary_color())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.get_text_color());
    let bar_style = Style::default()
        .fg(theme.get_primary_color())
        .add_modifier(Modifier::BOLD);
    let pct_style = Style::default()
        .fg(theme.get_secondary_color())
        .add_modifier(Modifier::BOLD);

    let inner_w = max_width.saturating_sub(4);
    let label_col = 14usize;
    let pct_col = 6usize;
    let bar_max = inner_w
        .saturating_sub(label_col)
        .saturating_sub(pct_col)
        .min(30);
    let bar_max = bar_max.max(10);

    let total: f64 = diagram.slices.iter().map(|(_, v)| v).sum();
    let max_val = diagram
        .slices
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(ref title) = diagram.title {
        let tw = unicode_width::UnicodeWidthStr::width(title.as_str());
        let pad = inner_w.saturating_sub(tw);
        let left_pad = pad / 2;
        let right_pad = pad - left_pad;
        lines.push(Line::from(vec![
            Span::styled(" ".repeat(left_pad), title_style),
            Span::styled(title.clone(), title_style),
            Span::styled(" ".repeat(right_pad), title_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            " ".repeat(inner_w),
            Style::default(),
        )]));
    }

    for (label, value) in &diagram.slices {
        let pct = if total > 0.0 {
            value / total * 100.0
        } else {
            0.0
        };
        let bar_len = if max_val > 0.0 {
            ((value / max_val) * bar_max as f64).round() as usize
        } else {
            0
        };
        let bar_len = bar_len.min(bar_max);

        let label_display = if label.len() > label_col {
            let mut end = label_col;
            while end > 0 && !label.is_char_boundary(end) {
                end -= 1;
            }
            label[..end].to_string()
        } else {
            label.clone()
        };
        let label_w = unicode_width::UnicodeWidthStr::width(label_display.as_str());
        let label_pad = label_col.saturating_sub(label_w);

        let pct_str = format!("{:.0}%", pct);
        let pct_w = pct_str.len();

        let mut bar_str = BLOCK.repeat(bar_len);
        let bg_len = bar_max.saturating_sub(bar_len);
        if bg_len > 0 {
            bar_str.push_str(&LIGHT_BLOCK.repeat(bg_len));
        }

        lines.push(Line::from(vec![
            Span::styled(" ".to_string(), label_style),
            Span::styled(label_display, label_style),
            Span::styled(" ".repeat(label_pad), label_style),
            Span::styled(" ".to_string(), label_style),
            Span::styled(bar_str, bar_style),
            Span::styled(" ".to_string(), label_style),
            Span::styled(pct_str, pct_style),
            Span::styled(" ".repeat(pct_col.saturating_sub(pct_w)), pct_style),
        ]));
    }

    lines
}

#[cfg(test)]
mod grammar_tests {
    use super::*;

    #[test]
    fn colon_inside_quoted_label_stays_in_the_label() {
        let chart = parse_pie("pie\n    \"Dogs: big\" : 10\n    \"Cats\" : 5").unwrap();
        assert_eq!(chart.slices[0].0, "Dogs: big");
        assert_eq!(chart.slices[0].1, 10.0);
        assert_eq!(chart.slices[1].0, "Cats");
    }
}
