#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    Heading1(String),
    Heading2(String),
    Heading3(String),
    Paragraph(Vec<String>),
    CodeBlock {
        lang: String,
        code: String,
        header_override: Option<String>,
        footer_override: Option<String>,
        prefix_override: Option<String>,
    },
    ListItem(String, u8),
    TaskItem {
        text: String,
        indent: u8,
        checked: bool,
    },
    Blockquote {
        level: u8,
        children: Vec<Self>,
        header_override: Option<String>,
        footer_override: Option<String>,
    },
    HorizontalRule,
    BlankLine,
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        alt: String,
        path: String,
    },
}

impl MarkdownBlock {
    pub fn code_block(lang: impl Into<String>, code: impl Into<String>) -> Self {
        Self::CodeBlock {
            lang: lang.into(),
            code: code.into(),
            header_override: None,
            footer_override: None,
            prefix_override: None,
        }
    }

}

#[derive(Debug)]
pub(crate) enum TextToken {
    Word(String),
    Space,
    Newline,
}
