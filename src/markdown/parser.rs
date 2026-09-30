#[cfg(feature = "image")]
use super::image::ImageResolver;
use super::{types::MarkdownBlock, MarkdownRenderer};

const MD_FENCE: &str = "```";

fn is_line_only_image(text: &str) -> Option<(String, String)> {
    let (alt, path, line_only) = scan_image(text)?;
    if line_only {
        Some((alt, path))
    } else {
        None
    }
}

fn scan_image(text: &str) -> Option<(String, String, bool)> {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();
    if bytes.first() != Some(&b'!') || bytes.get(1) != Some(&b'[') {
        return None;
    }
    let alt_end = find_balanced(trimmed, 1, b'[', b']')?;
    let alt = unescape_markdown(&trimmed[2..alt_end]);
    let after_alt = alt_end + 1;
    if bytes.get(after_alt) != Some(&b'(') {
        return None;
    }
    let dest_end = find_balanced(trimmed, after_alt, b'(', b')')?;
    let destination = trimmed[after_alt + 1..dest_end].trim();
    let (path, consumed) = split_image_destination(destination)?;
    if path.is_empty() {
        return None;
    }
    let rest = destination[consumed..].trim();
    if !rest.is_empty() && !is_markdown_title(rest) {
        return None;
    }
    let line_only = trimmed[dest_end + 1..].trim().is_empty();
    Some((alt, path, line_only))
}

fn find_balanced(text: &str, open_at: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open_at) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for (idx, &byte) in bytes.iter().enumerate().skip(open_at) {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn unescape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            } else {
                out.push('\\');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn split_image_destination(destination: &str) -> Option<(String, usize)> {
    let dest = destination.trim_start();
    let leading = destination.len() - dest.len();
    if dest.is_empty() {
        return None;
    }
    let bytes = dest.as_bytes();
    if bytes[0] == b'<' {
        let mut escaped = false;
        for (idx, &byte) in bytes.iter().enumerate().skip(1) {
            if escaped {
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                continue;
            }
            if byte == b'>' {
                return Some((unescape_markdown(&dest[1..idx]), leading + idx + 1));
            }
            if byte.is_ascii_whitespace() || byte == b'<' {
                return None;
            }
        }
        return None;
    }
    let mut escaped = false;
    let mut end = 0usize;
    for (idx, &byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            end = idx + 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            end = idx + 1;
            continue;
        }
        if byte.is_ascii_whitespace() {
            break;
        }
        if byte == b'(' || byte == b')' {
            return None;
        }
        end = idx + 1;
    }
    if end == 0 {
        return None;
    }
    Some((unescape_markdown(&dest[..end]), leading + end))
}

fn is_markdown_title(text: &str) -> bool {
    let bytes = text.as_bytes();
    let close = match bytes.first() {
        Some(b'"') => b'"',
        Some(b'\'') => b'\'',
        Some(b'(') => b')',
        _ => return false,
    };
    bytes.len() >= 2 && *bytes.last().unwrap_or(&0) == close
}

fn is_thematic_break(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && i < 3 && bytes[i] == b' ' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'\t' {
        return false;
    }
    let rest = line[i..].trim_end();
    if rest.len() < 3 {
        return false;
    }
    let marker = rest.as_bytes()[0];
    if marker != b'-' && marker != b'*' && marker != b'_' {
        return false;
    }
    let mut count = 0usize;
    for byte in rest.as_bytes() {
        if *byte == marker {
            count += 1;
        } else if *byte != b' ' && *byte != b'\t' {
            return false;
        }
    }
    count >= 3
}

fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && i < 3 && bytes[i] == b' ' {
        i += 1;
    }
    if bytes.get(i) != Some(&b'#') {
        return None;
    }
    let mut level = 0u8;
    while bytes.get(i) == Some(&b'#') {
        level += 1;
        i += 1;
        if level > 6 {
            return None;
        }
    }
    if i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' {
        return None;
    }
    let mut content = line[i..].trim().to_string();
    if let Some(stripped) = strip_atx_closing(&content) {
        content = stripped;
    }
    Some((level, content))
}

fn strip_atx_closing(content: &str) -> Option<String> {
    let trimmed = content.trim_end();
    if !trimmed.ends_with('#') {
        return None;
    }
    let hash_start = trimmed.trim_end_matches('#').len();
    if hash_start == trimmed.len() {
        return None;
    }
    if hash_start > 0 {
        let before = trimmed.as_bytes()[hash_start - 1];
        if before != b' ' && before != b'\t' {
            return None;
        }
        if hash_start >= 2 && trimmed.as_bytes()[hash_start - 2] == b'\\' {
            return None;
        }
    }
    Some(trimmed[..hash_start].trim_end().to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ListMarkerKind {
    Bullet,
    Ordered,
    Task { checked: bool },
}

struct ListMarker {
    kind: ListMarkerKind,
    content: String,
}

fn parse_list_marker(line: &str) -> Option<ListMarker> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut spaces = 0usize;
    while i < bytes.len() && bytes[i] == b' ' && spaces < 3 {
        i += 1;
        spaces += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let marker = bytes[i];
    if marker == b'-' || marker == b'*' || marker == b'+' {
        let after = i + 1;
        if after < bytes.len() && bytes[after] != b' ' && bytes[after] != b'\t' {
            return None;
        }
        let content = line.get(after..).unwrap_or("").trim_start().to_string();
        return Some(task_or_bullet(content));
    }
    if !marker.is_ascii_digit() {
        return None;
    }
    let num_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() && i - num_start < 9 {
        i += 1;
    }
    if i == num_start || i >= bytes.len() || (bytes[i] != b'.' && bytes[i] != b')') {
        return None;
    }
    let after = i + 1;
    if after < bytes.len() && bytes[after] != b' ' && bytes[after] != b'\t' {
        return None;
    }
    Some(ListMarker {
        kind: ListMarkerKind::Ordered,
        content: line.get(after..).unwrap_or("").trim_start().to_string(),
    })
}

fn task_or_bullet(content: String) -> ListMarker {
    let bytes = content.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'[' && bytes[2] == b']' {
        let mark = bytes[1];
        if mark == b' ' || mark == b'x' || mark == b'X' {
            let after = if bytes.len() == 3 {
                String::new()
            } else if bytes[3] == b' ' || bytes[3] == b'\t' {
                content[3..].trim_start().to_string()
            } else {
                return ListMarker {
                    kind: ListMarkerKind::Bullet,
                    content,
                };
            };
            return ListMarker {
                kind: ListMarkerKind::Task {
                    checked: mark == b'x' || mark == b'X',
                },
                content: after,
            };
        }
    }
    ListMarker {
        kind: ListMarkerKind::Bullet,
        content,
    }
}

fn is_table_delimiter_row(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut cells = 0usize;
    let mut cell_start = 0usize;
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    let mut escaped = false;
    let mut code_ticks = 0usize;
    while i <= bytes.len() {
        let at_sep = if i == bytes.len() {
            true
        } else {
            let byte = bytes[i];
            if escaped {
                escaped = false;
                false
            } else if byte == b'\\' {
                escaped = true;
                false
            } else if byte == b'`' {
                let mut run = 0usize;
                while i + run < bytes.len() && bytes[i + run] == b'`' {
                    run += 1;
                }
                if code_ticks == 0 {
                    code_ticks = run;
                } else if run == code_ticks {
                    code_ticks = 0;
                }
                i += run;
                continue;
            } else {
                byte == b'|' && code_ticks == 0
            }
        };
        if at_sep {
            let cell = trimmed[cell_start..i].trim();
            let skip = cell.is_empty() && (cells == 0 || i == bytes.len());
            if !skip {
                if !is_delimiter_cell(cell) {
                    return false;
                }
                cells += 1;
            }
            cell_start = i + 1;
        }
        i += 1;
    }
    cells >= 2
}

fn is_delimiter_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    if bytes[i] == b':' {
        i += 1;
    }
    let dash_start = i;
    while i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    if i == dash_start {
        return false;
    }
    if i < bytes.len() && bytes[i] == b':' {
        i += 1;
    }
    i == bytes.len()
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut separators = 0usize;
    let mut escaped = false;
    let mut code_ticks = 0usize;
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let byte = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            i += 1;
            continue;
        }
        if byte == b'`' {
            let mut run = 0usize;
            while i + run < bytes.len() && bytes[i + run] == b'`' {
                run += 1;
            }
            if code_ticks == 0 {
                code_ticks = run;
            } else if run == code_ticks {
                code_ticks = 0;
            }
            i += run;
            continue;
        }
        if byte == b'|' && code_ticks == 0 {
            separators += 1;
        }
        i += 1;
    }
    separators >= 2
}

fn split_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut code_ticks = 0usize;
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if escaped {
            if ch != '|' {
                current.push('\\');
            }
            current.push(ch);
            escaped = false;
            i += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            i += 1;
            continue;
        }
        if ch == '`' {
            let mut run = 0usize;
            while i + run < chars.len() && chars[i + run] == '`' {
                run += 1;
            }
            if code_ticks == 0 {
                code_ticks = run;
            } else if run == code_ticks {
                code_ticks = 0;
            }
            for _ in 0..run {
                current.push('`');
            }
            i += run;
            continue;
        }
        if ch == '|' && code_ticks == 0 {
            cells.push(current.trim().to_string());
            current.clear();
            i += 1;
            continue;
        }
        current.push(ch);
        i += 1;
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current.trim().to_string());
    if cells.first().is_some_and(|cell| cell.is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|cell| cell.is_empty()) {
        cells.pop();
    }
    cells
}

impl MarkdownRenderer {
    pub fn parse(&self, markdown: &str) -> Vec<MarkdownBlock> {
        self.parse_inner(markdown)
    }

    #[cfg(feature = "image")]
    pub fn parse_with_images<I: ImageResolver>(
        &self,
        markdown: &str,
        resolver: &mut I,
    ) -> (Vec<MarkdownBlock>, Vec<super::image::ResolvedImage>) {
        let blocks = self.parse_inner(markdown);
        let mut resolved = Vec::new();
        for block in &blocks {
            if let MarkdownBlock::Image { path, .. } = block {
                if let Some(img) = resolver.resolve(path) {
                    resolved.push(super::image::ResolvedImage {
                        path: path.clone(),
                        image: img,
                    });
                }
            }
        }
        (blocks, resolved)
    }

    fn parse_inner(&self, markdown: &str) -> Vec<MarkdownBlock> {
        self.parse_lines(markdown.lines())
    }

    fn parse_lines<'a>(&self, lines: impl Iterator<Item = &'a str>) -> Vec<MarkdownBlock> {
        let mut blocks = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();
        let mut paragraph_lines: Vec<String> = Vec::new();
        let mut table_buffer: Vec<String> = Vec::new();

        let mut lines = lines.peekable();

        while let Some(line) = lines.next() {
            if in_code_block {
                if line.trim().starts_with(MD_FENCE) {
                    in_code_block = false;
                    blocks.push(MarkdownBlock::code_block(
                        code_lang.clone(),
                        code_content.trim_end(),
                    ));
                    code_lang.clear();
                    code_content.clear();
                } else {
                    code_content.push_str(line);
                    code_content.push('\n');
                }
                continue;
            }

            if line.trim().starts_with(MD_FENCE) {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                in_code_block = true;
                code_lang = line.trim().chars().skip(3).collect::<String>();
                continue;
            }

            let trimmed = line.trim();

            if trimmed.is_empty() {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                blocks.push(MarkdownBlock::BlankLine);
                continue;
            }

            if let Some((alt, path)) = is_line_only_image(trimmed) {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                blocks.push(MarkdownBlock::Image { alt, path });
                continue;
            }

            if is_thematic_break(line) {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                blocks.push(MarkdownBlock::HorizontalRule);
                continue;
            }

            if let Some((level, text)) = parse_atx_heading(line) {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                blocks.push(match level {
                    1 => MarkdownBlock::Heading1(text),
                    2 => MarkdownBlock::Heading2(text),
                    _ => MarkdownBlock::Heading3(text),
                });
                continue;
            }

            if trimmed.starts_with('>') {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                let mut bq_lines: Vec<String> = Vec::new();
                bq_lines.push(trimmed.to_string());

                while let Some(&next) = lines.peek() {
                    let next_trimmed = next.trim();
                    if next_trimmed.is_empty() || !next_trimmed.starts_with('>') {
                        break;
                    }
                    bq_lines.push(next_trimmed.to_string());
                    lines.next();
                }

                let blockquote = Self::parse_blockquote_group(&bq_lines);
                blocks.push(blockquote);
                continue;
            }

            let list_indent = Self::count_list_indent(line);
            if let Some(marker) = parse_list_marker(line) {
                Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                match marker.kind {
                    ListMarkerKind::Task { checked } => {
                        blocks.push(MarkdownBlock::TaskItem {
                            text: marker.content,
                            indent: list_indent,
                            checked,
                        });
                    }
                    ListMarkerKind::Bullet | ListMarkerKind::Ordered => {
                        blocks.push(MarkdownBlock::ListItem(marker.content, list_indent));
                    }
                }
                continue;
            }

            if is_table_row(line) {

                if !paragraph_lines.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut paragraph_lines)));
                }
                table_buffer.push(trimmed.to_string());
                continue;
            }

            Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
            paragraph_lines.push(trimmed.to_string());
        }

        Self::flush_table(&mut table_buffer, &mut blocks, &mut paragraph_lines);
        if !paragraph_lines.is_empty() {
            blocks.push(MarkdownBlock::Paragraph(paragraph_lines));
        }

        if in_code_block {
            blocks.push(MarkdownBlock::code_block(
                code_lang,
                code_content.trim_end(),
            ));
        }

        blocks
    }

    fn parse_blockquote_group(lines: &[String]) -> MarkdownBlock {
        let mut max_level: u8 = 1;
        let mut inner_lines: Vec<(u8, String)> = Vec::new();

        for line in lines {
            let (level, content) = Self::strip_blockquote_prefix(line);
            if level > max_level {
                max_level = level;
            }
            inner_lines.push((level, content));
        }

        if max_level == 1 {
            let children = Self::parse_blockquote_content(inner_lines);
            return MarkdownBlock::Blockquote {
                level: 1,
                children,
                header_override: None,
                footer_override: None,
            };
        }

        let children = Self::parse_nested_blockquote(inner_lines, 1);
        MarkdownBlock::Blockquote {
            level: 1,
            children,
            header_override: None,
            footer_override: None,
        }
    }

    fn strip_blockquote_prefix(line: &str) -> (u8, String) {
        let mut level: u8 = 0;
        let rest = line.trim_start();
        let bytes = rest.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() && bytes[i] == b'>' {
            level += 1;
            i += 1;
            if i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
        }
        (level, rest[i..].to_string())
    }

    fn parse_nested_blockquote(mut lines: Vec<(u8, String)>, current_level: u8) -> Vec<MarkdownBlock> {
        let mut children = Vec::new();
        while !lines.is_empty() {
            let deeper = lines[0].0 > current_level;
            let mut end = 1usize;
            while end < lines.len() && (lines[end].0 > current_level) == deeper {
                end += 1;
            }
            let rest = lines.split_off(end);
            Self::push_blockquote_run(lines, deeper, current_level, &mut children);
            lines = rest;
        }
        children
    }

    fn push_blockquote_run(
        run: Vec<(u8, String)>,
        deeper: bool,
        current_level: u8,
        children: &mut Vec<MarkdownBlock>,
    ) {
        if deeper {
            children.push(Self::parse_nested_blockquote_inner(run, current_level + 1));
        } else {
            children.extend(Self::parse_blockquote_content(run));
        }
    }

    fn parse_nested_blockquote_inner(lines: Vec<(u8, String)>, target_level: u8) -> MarkdownBlock {
        let has_deeper = lines.iter().any(|(level, _)| *level > target_level);
        let children = if has_deeper {
            Self::parse_nested_blockquote(lines, target_level)
        } else {
            Self::parse_blockquote_content(lines)
        };
        MarkdownBlock::Blockquote {
            level: target_level,
            children,
            header_override: None,
            footer_override: None,
        }
    }

    fn parse_blockquote_content(lines: Vec<(u8, String)>) -> Vec<MarkdownBlock> {
        let renderer = MarkdownRenderer::new(usize::MAX);
        let mut blocks = renderer.parse_lines(lines.iter().map(|(_, content)| content.as_str()));
        blocks.retain(|block| !matches!(block, MarkdownBlock::BlankLine));
        if blocks.is_empty() {
            let all_text = lines
                .into_iter()
                .filter_map(|(_, content)| if content.is_empty() { None } else { Some(content) })
                .collect::<Vec<_>>();
            if !all_text.is_empty() {
                blocks.push(MarkdownBlock::Paragraph(all_text));
            }
        }
        blocks
    }

    fn flush_table(
        table_buffer: &mut Vec<String>,
        blocks: &mut Vec<MarkdownBlock>,
        paragraph_lines: &mut Vec<String>,
    ) {
        if table_buffer.is_empty() {
            return;
        }
        let mut separator_idx = None;
        let mut split_rows = Vec::with_capacity(table_buffer.len());
        for (idx, line) in table_buffer.iter().enumerate() {
            if is_table_delimiter_row(line) {
                if separator_idx.is_none() {
                    separator_idx = Some(idx);
                }
                split_rows.push(Vec::new());
            } else {
                split_rows.push(split_table_cells(line));
            }
        }
        if table_buffer.len() < 2 || separator_idx.is_none() || separator_idx == Some(0) {
            for line in table_buffer.drain(..) {
                paragraph_lines.push(line);
            }
            return;
        }
        let sep_pos = separator_idx.unwrap_or(0);
        let headers = std::mem::take(&mut split_rows[sep_pos - 1]);
        let rows = split_rows
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| *idx > sep_pos)
            .map(|(_, cells)| cells)
            .filter(|cells| !cells.is_empty())
            .collect();
        blocks.push(MarkdownBlock::Table { headers, rows });
        table_buffer.clear();
    }

    fn count_list_indent(line: &str) -> u8 {
        let spaces = line.chars().take_while(|&c| c == ' ').count();
        (spaces / 2).min(255) as u8
    }
}

#[cfg(test)]
mod grammar_tests {
    use super::*;

    fn parse(markdown: &str) -> Vec<MarkdownBlock> {
        MarkdownRenderer::new(80).parse(markdown)
    }

    #[test]
    fn image_balances_brackets_and_honors_escapes() {
        let blocks = parse("![alt](img.png)");
        assert!(matches!(
            &blocks[0],
            MarkdownBlock::Image { alt, path } if alt == "alt" && path == "img.png"
        ));
        let escaped = parse("![a\\]b](img.png)");
        assert!(matches!(
            &escaped[0],
            MarkdownBlock::Image { alt, path } if alt == "a]b" && path == "img.png"
        ));
        assert!(matches!(&parse("![alt](img.png")[0], MarkdownBlock::Paragraph(_)));
        assert!(matches!(
            &parse("![alt](img.png \"title\")")[0],
            MarkdownBlock::Image { path, .. } if path == "img.png"
        ));
    }

    #[test]
    fn thematic_break_follows_commonmark_not_prefix() {
        assert_eq!(parse("---"), vec![MarkdownBlock::HorizontalRule]);
        assert_eq!(parse("  ***"), vec![MarkdownBlock::HorizontalRule]);
        assert_eq!(parse("- - -"), vec![MarkdownBlock::HorizontalRule]);
        assert!(matches!(&parse("--- not a rule")[0], MarkdownBlock::Paragraph(_)));
        assert!(matches!(&parse("----title")[0], MarkdownBlock::Paragraph(_)));
    }

    #[test]
    fn atx_heading_allows_indent_and_strips_closing() {
        assert_eq!(parse("# Hello"), vec![MarkdownBlock::Heading1("Hello".into())]);
        assert_eq!(parse("  ## Section ##"), vec![MarkdownBlock::Heading2("Section".into())]);
        assert_eq!(parse("### Sub #"), vec![MarkdownBlock::Heading3("Sub".into())]);
        assert!(matches!(&parse("#not a heading")[0], MarkdownBlock::Paragraph(_)));
        assert!(matches!(&parse("    # indented too far")[0], MarkdownBlock::Paragraph(_)));
    }

    #[test]
    fn lists_recognize_markers_tasks_and_ordered() {
        assert!(matches!(&parse("- item")[0], MarkdownBlock::ListItem(text, _) if text == "item"));
        assert!(matches!(&parse("- [x] done")[0], MarkdownBlock::TaskItem { text, checked: true, .. } if text == "done"));
        assert!(matches!(&parse("1. first")[0], MarkdownBlock::ListItem(text, _) if text == "first"));
        assert!(matches!(&parse("12) twelfth")[0], MarkdownBlock::ListItem(text, _) if text == "twelfth"));
        assert!(matches!(&parse("1.2 not a list")[0], MarkdownBlock::Paragraph(_)));
        assert!(matches!(&parse("not. a list")[0], MarkdownBlock::Paragraph(_)));
    }

    #[test]
    fn blockquote_reuses_block_parser() {
        let blocks = parse("> # Title\n> - [ ] task\n> 1. ordered\n> | A | B |\n> | --- | --- |\n> | 1 | 2 |");
        let MarkdownBlock::Blockquote { children, .. } = &blocks[0] else {
            panic!("expected blockquote, got {blocks:?}");
        };
        assert!(matches!(&children[0], MarkdownBlock::Heading1(text) if text == "Title"));
        assert!(matches!(&children[1], MarkdownBlock::TaskItem { checked: false, .. }));
        assert!(matches!(&children[2], MarkdownBlock::ListItem(text, _) if text == "ordered"));
        assert!(matches!(&children[3], MarkdownBlock::Table { .. }));
    }

    #[test]
    fn nested_blockquote_reuses_block_parser() {
        let blocks = parse("> > # Title\n> > - [ ] task\n> > 1. ordered\n> > | A | B |\n> > | --- | --- |\n> > | 1 | 2 |");
        let MarkdownBlock::Blockquote { children, .. } = &blocks[0] else {
            panic!("expected outer blockquote, got {blocks:?}");
        };
        let MarkdownBlock::Blockquote { children: inner, level, .. } = &children[0] else {
            panic!("expected nested blockquote, got {children:?}");
        };
        assert_eq!(*level, 2);
        assert!(matches!(&inner[0], MarkdownBlock::Heading1(text) if text == "Title"));
        assert!(matches!(&inner[1], MarkdownBlock::TaskItem { checked: false, .. }));
        assert!(matches!(&inner[2], MarkdownBlock::ListItem(text, _) if text == "ordered"));
        assert!(matches!(&inner[3], MarkdownBlock::Table { .. }));
    }

    #[test]
    fn tables_require_delimiter_and_keep_escaped_pipes() {
        let blocks = parse("| A | B |\n| --- | --- |\n| a\\|b | `x|y` |");
        let MarkdownBlock::Table { headers, rows } = &blocks[0] else {
            panic!("expected table, got {blocks:?}");
        };
        assert_eq!(headers, &vec!["A".to_string(), "B".to_string()]);
        assert_eq!(rows[0][0], "a|b");
        assert_eq!(rows[0][1], "`x|y`");
        assert!(matches!(&parse("| just | pipes |")[0], MarkdownBlock::Paragraph(_)));
    }
}
