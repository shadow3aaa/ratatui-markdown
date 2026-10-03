use ratatui::text::Line;
use unicode_width::UnicodeWidthChar;

use super::{
    graph, layout, render,
    types::{Direction, EdgeType, MermaidDiagram, MermaidEdge, MermaidNode, NodeShape},
};
use crate::theme::RichTextTheme;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDefinition {
    pub name: String,
    pub attributes: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassMember {
    pub visibility: Visibility,
    pub name: String,
    pub type_info: String,
    pub is_method: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelationshipType {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Implements,
    DirectedAssociation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassRelationship {
    pub from: String,
    pub to: String,
    pub rel_type: RelationshipType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDiagram {
    pub classes: Vec<ClassDefinition>,
    pub relationships: Vec<ClassRelationship>,
}

fn render_relationship_label(r: &RelationshipType) -> &'static str {
    match r {
        RelationshipType::Inheritance => "extends",
        RelationshipType::Composition => "has",
        RelationshipType::Aggregation => "has",
        RelationshipType::Association => "uses",
        RelationshipType::Dependency => "depends",
        RelationshipType::Implements => "impl",
        RelationshipType::DirectedAssociation => "uses",
    }
}

#[allow(dead_code)]
fn visibility_char(v: Visibility) -> char {
    match v {
        Visibility::Public => '+',
        Visibility::Private => '-',
        Visibility::Protected => '#',
        Visibility::Internal => '~',
    }
}

fn relationship_to_edge_type(r: RelationshipType) -> EdgeType {
    match r {
        RelationshipType::Inheritance => EdgeType::Arrow,
        RelationshipType::Composition => EdgeType::Arrow,
        RelationshipType::Aggregation => EdgeType::Arrow,
        RelationshipType::Association => EdgeType::Line,
        RelationshipType::Dependency => EdgeType::Line,
        RelationshipType::Implements => EdgeType::Arrow,
        RelationshipType::DirectedAssociation => EdgeType::Arrow,
    }
}

fn unicode_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

fn build_class_label(class: &ClassDefinition, max_width: usize) -> String {
    let title_content = format!(" {} ", class.name);
    let title_w = unicode_width(&title_content);

    let attr_texts: Vec<String> = class
        .attributes
        .iter()
        .map(|a| {
            let v = visibility_char(a.visibility);
            if a.type_info.is_empty() {
                format!("{} {}", v, a.name)
            } else {
                format!("{} {}: {}", v, a.name, a.type_info)
            }
        })
        .collect();

    let method_texts: Vec<String> = class
        .methods
        .iter()
        .map(|m| {
            let v = visibility_char(m.visibility);
            if m.type_info.is_empty() {
                format!("{} {}()", v, m.name)
            } else {
                format!("{} {}(): {}", v, m.name, m.type_info)
            }
        })
        .collect();

    let max_content = title_w
        .max(
            attr_texts
                .iter()
                .map(|t| unicode_width(t) + 1)
                .max()
                .unwrap_or(0),
        )
        .max(
            method_texts
                .iter()
                .map(|t| unicode_width(t) + 1)
                .max()
                .unwrap_or(0),
        );

    let available = max_content.max(max_width.saturating_sub(4)).clamp(4, 60);
    let sep = "\u{2500}".repeat(available);

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("\u{250c}{}\u{2510}", sep));

    let pad = available.saturating_sub(title_w);
    let left = pad / 2;
    let right = pad - left;
    lines.push(format!(
        "\u{2502}{}{}{}\u{2502}",
        " ".repeat(left),
        title_content,
        " ".repeat(right)
    ));

    lines.push(format!("\u{251c}{}\u{2524}", sep));

    for text in &attr_texts {
        let tw = unicode_width(text);
        let pad = available.saturating_sub(tw + 1);
        lines.push(format!("\u{2502} {}{}\u{2502}", text, " ".repeat(pad)));
    }

    if !class.attributes.is_empty() && !class.methods.is_empty() {
        lines.push(format!("\u{2502}{}\u{2502}", " ".repeat(available)));
    }

    for text in &method_texts {
        let tw = unicode_width(text);
        let pad = available.saturating_sub(tw + 1);
        lines.push(format!("\u{2502} {}{}\u{2502}", text, " ".repeat(pad)));
    }

    lines.push(format!("\u{2514}{}\u{2518}", sep));
    lines.join("\n")
}

pub fn parse_class_diagram(source: &str) -> Option<ClassDiagram> {
    let mut classes: Vec<ClassDefinition> = Vec::new();
    let mut relationships: Vec<ClassRelationship> = Vec::new();
    let mut current_class: Option<ClassDefinition> = None;
    let mut in_class_body = false;
    let mut brace_depth = 0;

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("classDiagram") {
            continue;
        }

        if let Some(ref mut class) = current_class {
            if in_class_body {
                let trimmed = line.trim();
                if trimmed == "}" {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_class_body = false;
                        classes.push(class.clone());
                        current_class = None;
                        continue;
                    }
                }
                if brace_depth > 0 && !trimmed.is_empty() && !trimmed.starts_with('%') {
                    let member = parse_member(trimmed);
                    if let Some(m) = member {
                        if m.is_method {
                            class.methods.push(m);
                        } else {
                            class.attributes.push(m);
                        }
                    }
                }
                continue;
            }
        } else if let Some(opened) =
            super::parser::parse_rule(super::parser::Rule::class_open, line)
        {
            if let Some(name) = super::parser::text_of(&opened, super::parser::Rule::class_name) {
                if !name.is_empty() {
                    in_class_body = true;
                    brace_depth = 1;
                    current_class = Some(ClassDefinition {
                        name,
                        attributes: Vec::new(),
                        methods: Vec::new(),
                    });
                }
            }
        } else if let Some(header) =
            super::parser::parse_rule(super::parser::Rule::class_header, line)
        {
            push_class_header(
                header,
                &mut current_class,
                &mut in_class_body,
                &mut brace_depth,
                &mut classes,
            );
        } else {
            let rel = parse_relationship(line);
            if let Some(r) = rel {
                relationships.push(r);
            }
        }
    }

    if current_class.is_some() {
        if let Some(class) = current_class.take() {
            if !class.attributes.is_empty() || !class.methods.is_empty() {
                classes.push(class);
            }
        }
    }

    if classes.is_empty() {
        return None;
    }

    Some(ClassDiagram {
        classes,
        relationships,
    })
}

fn push_class_header(
    header: pest::iterators::Pair<'_, super::parser::Rule>,
    current_class: &mut Option<ClassDefinition>,
    in_class_body: &mut bool,
    brace_depth: &mut i32,
    classes: &mut Vec<ClassDefinition>,
) {
    let Some(name) = super::parser::text_of(&header, super::parser::Rule::class_name) else {
        return;
    };
    if name.is_empty() {
        return;
    }
    *in_class_body = true;
    *brace_depth = 1;
    *current_class = Some(ClassDefinition {
        name,
        attributes: Vec::new(),
        methods: Vec::new(),
    });
    let Some(body) = super::parser::text_of(&header, super::parser::Rule::class_body_inner) else {
        return;
    };
    let inline = body.trim();
    if inline.is_empty() {
        return;
    }
    for member_str in inline.split(';') {
        let trimmed = member_str.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(m) = parse_member(trimmed) {
            if let Some(class) = current_class.as_mut() {
                if m.is_method {
                    class.methods.push(m);
                } else {
                    class.attributes.push(m);
                }
            }
        }
    }
    *brace_depth = 0;
    *in_class_body = false;
    if let Some(class) = current_class.take() {
        classes.push(class);
    }
}

fn parse_member(line: &str) -> Option<ClassMember> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('%') {
        return None;
    }

    let Some(parsed) = super::parser::parse_rule(super::parser::Rule::class_member, line) else {
        return Some(ClassMember {
            visibility: Visibility::Internal,
            name: line.to_string(),
            type_info: String::new(),
            is_method: line.contains('('),
        });
    };

    let vis = super::parser::child(&parsed, super::parser::Rule::vis)?;
    let visibility = match vis.as_str() {
        "+" => Visibility::Public,
        "-" => Visibility::Private,
        "#" => Visibility::Protected,
        "~" => Visibility::Internal,
        _ => Visibility::Internal,
    };
    let rest =
        super::parser::text_of(&parsed, super::parser::Rule::member_rest).unwrap_or_default();
    let rest = rest.trim();
    let is_method = rest.contains('(');

    if is_method {
        let (name, type_info) = if let Some(paren) = rest.find('(') {
            let name = rest[..paren].trim();
            let after_paren = &rest[paren..];
            let type_info = if let Some(colon) = after_paren.rfind(':') {
                after_paren[colon + 1..].trim().to_string()
            } else {
                String::new()
            };
            (name.to_string(), type_info)
        } else {
            (rest.to_string(), String::new())
        };
        Some(ClassMember {
            visibility,
            name,
            type_info,
            is_method: true,
        })
    } else {
        let (name, type_info) = if let Some(colon) = rest.find(':') {
            let name = rest[..colon].trim().to_string();
            let t = rest[colon + 1..].trim().to_string();
            (name, t)
        } else {
            (rest.to_string(), String::new())
        };
        Some(ClassMember {
            visibility,
            name,
            type_info,
            is_method: false,
        })
    }
}

fn parse_relationship(line: &str) -> Option<ClassRelationship> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('%') {
        return None;
    }

    let parsed = super::parser::parse_rule(super::parser::Rule::class_rel, line)?;
    let ends = super::parser::children(&parsed, super::parser::Rule::class_end);
    if ends.len() != 2 {
        return None;
    }
    let op = super::parser::child(&parsed, super::parser::Rule::class_op)?;
    let rel_type = relationship_of(&op)?;
    let from = class_end_text(&ends[0]);
    let to = class_end_text(&ends[1]);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let label = super::parser::text_of(&parsed, super::parser::Rule::class_label)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Some(ClassRelationship {
        from,
        to,
        rel_type,
        label,
    })
}

fn class_end_text(end: &pest::iterators::Pair<'_, super::parser::Rule>) -> String {
    super::parser::text_of(end, super::parser::Rule::qinner)
        .or_else(|| super::parser::text_of(end, super::parser::Rule::class_name))
        .unwrap_or_else(|| end.as_str().trim().trim_matches('"').to_string())
}

fn relationship_of(
    op: &pest::iterators::Pair<'_, super::parser::Rule>,
) -> Option<RelationshipType> {
    let inner = op.clone().into_inner().next()?;
    Some(match inner.as_rule() {
        super::parser::Rule::op_inherit => RelationshipType::Inheritance,
        super::parser::Rule::op_implements => RelationshipType::Implements,
        super::parser::Rule::op_comp => RelationshipType::Composition,
        super::parser::Rule::op_agg => RelationshipType::Aggregation,
        super::parser::Rule::op_dep => RelationshipType::Dependency,
        super::parser::Rule::op_dir_left | super::parser::Rule::op_dir_right => {
            RelationshipType::DirectedAssociation
        }
        super::parser::Rule::op_assoc => RelationshipType::Association,
        _ => return None,
    })
}

pub fn convert_to_mermaid_diagram(class_diagram: &ClassDiagram) -> MermaidDiagram {
    use std::collections::HashMap;

    let mut nodes: Vec<MermaidNode> = Vec::new();
    let mut node_map: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<MermaidEdge> = Vec::new();

    let max_label_width: usize = class_diagram
        .classes
        .iter()
        .map(|c| {
            let name_w = c.name.len();
            let attr_w = c
                .attributes
                .iter()
                .map(|a| a.name.len() + a.type_info.len() + 5)
                .max()
                .unwrap_or(0);
            let meth_w = c
                .methods
                .iter()
                .map(|m| m.name.len() + m.type_info.len() + 7)
                .max()
                .unwrap_or(0);
            (name_w + 4).max(attr_w).max(meth_w)
        })
        .max()
        .unwrap_or(20)
        .min(40);

    for class in &class_diagram.classes {
        let label = build_class_label(class, max_label_width);
        MermaidDiagram::ensure_node(
            &mut nodes,
            &mut node_map,
            &class.name,
            Some(&label),
            Some(NodeShape::Rect),
        );
    }

    for rel in &class_diagram.relationships {
        let edge_type = relationship_to_edge_type(rel.rel_type);
        let label = rel.label.clone().or_else(|| {
            if rel.from != rel.to {
                Some(render_relationship_label(&rel.rel_type).to_string())
            } else {
                None
            }
        });
        edges.push(MermaidEdge {
            source: rel.from.clone(),
            target: rel.to.clone(),
            label,
            edge_type,
        });
    }

    MermaidDiagram {
        direction: Direction::TopDown,
        nodes,
        edges,
    }
}

pub fn render_class_diagram(
    source: &str,
    max_width: usize,
    max_height: Option<usize>,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let diagram = parse_class_diagram(source)?;
    let mermaid = convert_to_mermaid_diagram(&diagram);
    let direction = mermaid.direction.clone();
    let graph = graph::assign_layers(&mermaid);
    let layout = layout::compute_layout(&mermaid, &graph, max_width, max_height);
    Some(render::render_layout(&layout, &direction, theme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_parse_simple_class_diagram() -> Result<()> {
        let source =
            "classDiagram\nclass Animal {\n  +String name\n  +int age\n  +makeSound() void\n}\n";
        let diagram = parse_class_diagram(source)
            .ok_or_else(|| anyhow::anyhow!("failed to parse class diagram"))?;
        assert_eq!(diagram.classes.len(), 1);
        assert_eq!(diagram.classes[0].name, "Animal");
        assert_eq!(diagram.classes[0].attributes.len(), 2);
        assert_eq!(diagram.classes[0].methods.len(), 1);
        Ok(())
    }

    #[test]
    fn test_parse_relationship() -> Result<()> {
        let rel = parse_relationship("Animal <|-- Dog")
            .ok_or_else(|| anyhow::anyhow!("failed to parse relationship"))?;
        assert_eq!(rel.from, "Animal");
        assert_eq!(rel.to, "Dog");
        assert_eq!(rel.rel_type, RelationshipType::Inheritance);
        Ok(())
    }

    #[test]
    fn test_parse_relationship_with_label() -> Result<()> {
        let rel = parse_relationship("Animal <|-- Dog : extends")
            .ok_or_else(|| anyhow::anyhow!("failed to parse relationship"))?;
        assert_eq!(rel.from, "Animal");
        assert_eq!(rel.to, "Dog");
        assert_eq!(rel.label.as_deref(), Some("extends"));
        Ok(())
    }

    #[test]
    fn test_parse_composition() -> Result<()> {
        let rel = parse_relationship("Car *-- Engine")
            .ok_or_else(|| anyhow::anyhow!("failed to parse relationship"))?;
        assert_eq!(rel.from, "Car");
        assert_eq!(rel.to, "Engine");
        assert_eq!(rel.rel_type, RelationshipType::Composition);
        Ok(())
    }

    #[test]
    fn test_parse_implements() -> Result<()> {
        let rel = parse_relationship("Dog ..|> Runnable")
            .ok_or_else(|| anyhow::anyhow!("failed to parse relationship"))?;
        assert_eq!(rel.from, "Dog");
        assert_eq!(rel.to, "Runnable");
        assert_eq!(rel.rel_type, RelationshipType::Implements);
        Ok(())
    }

    #[test]
    fn test_convert_to_mermaid() -> Result<()> {
        let source = "classDiagram\nclass Animal {\n  +String name\n}\nclass Dog {\n  +String breed\n}\nAnimal <|-- Dog\n";
        let diagram = parse_class_diagram(source)
            .ok_or_else(|| anyhow::anyhow!("failed to parse class diagram"))?;
        let mermaid = convert_to_mermaid_diagram(&diagram);
        assert_eq!(mermaid.nodes.len(), 2);
        assert_eq!(mermaid.edges.len(), 1);
        Ok(())
    }

    #[test]
    fn directed_association_is_not_a_plain_association() -> Result<()> {
        let rel = parse_relationship("A --> B")
            .ok_or_else(|| anyhow::anyhow!("failed to parse relationship"))?;
        assert_eq!(rel.from, "A");
        assert_eq!(rel.to, "B");
        assert_eq!(rel.rel_type, RelationshipType::DirectedAssociation);

        let plain = parse_relationship("A -- B").unwrap();
        assert_eq!(plain.rel_type, RelationshipType::Association);

        let implements = parse_relationship("A ..|> B").unwrap();
        assert_eq!(implements.rel_type, RelationshipType::Implements);
        assert_ne!(implements.rel_type, RelationshipType::Dependency);
        Ok(())
    }
}
