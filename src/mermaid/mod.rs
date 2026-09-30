mod block;
mod class_diagram;
mod gantt;
mod graph;
mod layout;
mod parser;
mod pie;
mod quadrant;
mod render;
mod sequence;
#[cfg(test)]
mod tests;
pub mod theme;
mod types;

pub use block::{BlockDiagram, BlockEntry};
pub use class_diagram::{
    ClassDefinition, ClassDiagram, ClassMember, ClassRelationship, RelationshipType, Visibility,
};
pub use quadrant::{QuadrantChart, QuadrantPoint};
use ratatui::text::Line;
pub use types::{
    Direction, EdgeType, GanttChart, GanttSection, GanttTask, MermaidDiagram, MermaidEdge,
    MermaidNode, NodeShape, PieChart, SeqArrowKind, SequenceDiagram, SequenceMessage,
};

use crate::theme::RichTextTheme;

pub fn render_mermaid(
    source: &str,
    max_width: usize,
    max_height: Option<usize>,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let first_line = source.lines().next()?.trim();

    if first_line.starts_with("graph ") || first_line.starts_with("flowchart ") {
        render_flowchart(source, max_width, max_height, theme)
    } else if first_line == "sequenceDiagram" || first_line.starts_with("sequenceDiagram") {
        render_sequence_diagram(source, max_width, theme)
    } else if first_line.starts_with("pie") {
        render_pie_chart(source, max_width, theme)
    } else if first_line == "gantt" {
        render_gantt_chart(source, max_width, theme)
    } else if first_line.starts_with("stateDiagram") {
        render_state_diagram(source, max_width, max_height, theme)
    } else if first_line.starts_with("classDiagram") {
        class_diagram::render_class_diagram(source, max_width, max_height, theme)
    } else if first_line.starts_with("quadrantChart") {
        let chart = quadrant::parse_quadrant(source)?;
        Some(quadrant::render_quadrant(&chart, max_width, theme))
    } else if first_line.starts_with("block-beta")
        || first_line == "block"
        || first_line.starts_with("block ")
    {
        block::render_block_diagram(source, max_width, max_height, theme)
    } else {
        render_flowchart(source, max_width, max_height, theme)
    }
}

fn render_flowchart(
    source: &str,
    max_width: usize,
    max_height: Option<usize>,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let diagram = parser::parse(source).ok()?;
    let direction = diagram.direction.clone();
    let graph = graph::assign_layers(&diagram);
    let layout = layout::compute_layout(&diagram, &graph, max_width, max_height);
    let lines = render::render_layout(&layout, &direction, theme);
    Some(lines)
}

fn render_sequence_diagram(
    source: &str,
    max_width: usize,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let diagram = sequence::parse_sequence(source)?;
    Some(sequence::render_sequence(&diagram, max_width, theme))
}

fn render_pie_chart(
    source: &str,
    max_width: usize,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let chart = pie::parse_pie(source)?;
    Some(pie::render_pie(&chart, max_width, theme))
}

fn render_gantt_chart(
    source: &str,
    max_width: usize,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let chart = gantt::parse_gantt(source)?;
    Some(gantt::render_gantt(&chart, max_width, theme))
}

fn render_state_diagram(
    source: &str,
    max_width: usize,
    max_height: Option<usize>,
    theme: &impl RichTextTheme,
) -> Option<Vec<Line<'static>>> {
    let diagram = parse_state_diagram(source)?;
    let direction = diagram.direction.clone();
    let graph = graph::assign_layers(&diagram);
    let layout = layout::compute_layout(&diagram, &graph, max_width, max_height);
    let lines = render::render_layout(&layout, &direction, theme);
    Some(lines)
}

pub(crate) fn parse_state_diagram(source: &str) -> Option<MermaidDiagram> {
    use std::collections::HashSet;
    use types::{EdgeType, MermaidEdge, MermaidNode};

    let mut nodes: Vec<MermaidNode> = Vec::new();
    let mut edges: Vec<MermaidEdge> = Vec::new();
    let mut node_set: HashSet<String> = HashSet::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("stateDiagram")
            || line.starts_with("state ")
            || line.starts_with("note ")
        {
            continue;
        }

        let Some(parsed) = parser::parse_rule(parser::Rule::state_line, line) else {
            continue;
        };
        let ends = parser::children(&parsed, parser::Rule::state_end);
        let Some(edge) = parser::child(&parsed, parser::Rule::state_edge) else {
            continue;
        };
        if ends.len() != 2 {
            continue;
        }

        let label_text = parser::text_of(&edge, parser::Rule::state_ltxt).filter(|t| !t.is_empty());
        let arrow_is_line = parser::child(&edge, parser::Rule::state_ln).is_some()
            || parser::child(&edge, parser::Rule::state_ln_lbl).is_some();

        let _from_marker = parser::child(&ends[0], parser::Rule::state_marker).is_some();
        let to_marker = parser::child(&ends[1], parser::Rule::state_marker).is_some();
        let (from_id, from_label, from_shape) = state_endpoint(&ends[0], false);
        let (to_id, to_label, to_shape) = state_endpoint(&ends[1], to_marker);

        if !node_set.contains(&from_id) {
            node_set.insert(from_id.clone());
            nodes.push(MermaidNode {
                id: from_id.clone(),
                label: from_label,
                shape: from_shape,
            });
        }
        if !node_set.contains(&to_id) {
            node_set.insert(to_id.clone());
            let node = MermaidNode {
                id: to_id.clone(),
                label: to_label,
                shape: to_shape,
            };
            nodes.push(node);
        }

        let edge_type = if arrow_is_line {
            EdgeType::Line
        } else {
            EdgeType::Arrow
        };

        edges.push(MermaidEdge {
            source: from_id,
            target: to_id,
            label: label_text,
            edge_type,
        });
    }

    if nodes.is_empty() {
        return None;
    }

    Some(MermaidDiagram {
        direction: types::Direction::TopDown,
        nodes,
        edges,
    })
}

fn state_endpoint(
    pair: &pest::iterators::Pair<'_, parser::Rule>,
    end_marker: bool,
) -> (String, String, types::NodeShape) {
    if parser::child(pair, parser::Rule::state_marker).is_some() {
        let id = if end_marker { "__end__" } else { "__start__" };
        (
            id.to_string(),
            "\u{25cf}".to_string(),
            types::NodeShape::Circle,
        )
    } else {
        let id = parser::text_of(pair, parser::Rule::state_id).unwrap_or_default();
        (id.clone(), id, types::NodeShape::Rounded)
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_parse_simple_flowchart() -> anyhow::Result<()> {
        let diagram =
            parser::parse("graph TD\nA[Start] --> B[End]").map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(
            diagram.nodes.len(),
            2,
            "expected 2 nodes, got {:?}",
            diagram.nodes
        );
        assert_eq!(
            diagram.edges.len(),
            1,
            "expected 1 edge, got {:?}",
            diagram.edges
        );
        assert_eq!(diagram.direction, Direction::TopDown);
        Ok(())
    }

    #[test]
    fn test_parse_with_labels() -> anyhow::Result<()> {
        let diagram =
            parser::parse("graph TD\nA -->|yes| B").map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges[0].label.as_deref(), Some("yes"));
        Ok(())
    }

    #[test]
    fn test_parse_lr_direction() -> anyhow::Result<()> {
        let diagram = parser::parse("graph LR\nA --> B").map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(diagram.direction, Direction::LeftRight);
        Ok(())
    }

    #[test]
    fn test_parse_sequence_diagram() -> anyhow::Result<()> {
        let diagram = sequence::parse_sequence(
            "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi",
        )
        .ok_or_else(|| anyhow::anyhow!("failed to parse sequence diagram"))?;
        assert_eq!(diagram.participants.len(), 2);
        assert_eq!(diagram.messages.len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_pie_chart() -> anyhow::Result<()> {
        let chart = pie::parse_pie("pie title Pets\n    \"Dogs\" : 386\n    \"Cats\" : 85")
            .ok_or_else(|| anyhow::anyhow!("failed to parse pie chart"))?;
        assert_eq!(chart.title.as_deref(), Some("Pets"));
        assert_eq!(chart.slices.len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_gantt_chart() -> anyhow::Result<()> {
        let chart = gantt::parse_gantt(
            "gantt\ntitle Project\nsection Phase 1\nTask 1 :a1, 7d\nTask 2 :a2, after a1, 5d",
        )
        .ok_or_else(|| anyhow::anyhow!("failed to parse gantt chart"))?;
        assert_eq!(chart.title.as_deref(), Some("Project"));
        assert_eq!(chart.sections.len(), 1);
        assert_eq!(chart.sections[0].tasks.len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_state_diagram() -> anyhow::Result<()> {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running\n    Running --> Idle",
        )
        .ok_or_else(|| anyhow::anyhow!("failed to parse state diagram"))?;
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 3);
        Ok(())
    }

    #[test]
    fn arrow_inside_a_state_label_is_not_the_transition() -> anyhow::Result<()> {
        let diagram = parse_state_diagram("stateDiagram-v2\n    Idle -->|go --> next| Running")
            .ok_or_else(|| anyhow::anyhow!("failed to parse labeled state"))?;
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].source, "Idle");
        assert_eq!(diagram.edges[0].target, "Running");
        assert_eq!(diagram.edges[0].label.as_deref(), Some("go --> next"));
        assert_eq!(diagram.edges[0].edge_type, EdgeType::Arrow);
        Ok(())
    }
}
