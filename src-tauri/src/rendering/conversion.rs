//! DOM-backed deterministic conversion-feature cleanup.
//!
//! This operates on sanitized HTML. The existing html2text DOM is used so
//! table changes operate on parsed nodes;
//! string scanning remains reserved for the image marker boundary.

use std::rc::Rc;

use html2text::{Element, Handle, RcDom};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::{NodeOrText, TreeSink};
use html5ever::{local_name, ns, parse_fragment, QualName};

use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};

use super::{styles, RenderingError};

const MAX_DOM_DEPTH: usize = 1024;

pub(crate) fn apply(
    body: &str,
    settings: ConversionSettings,
    mode: ConversionSettingsApplicationMode,
) -> Result<String, RenderingError> {
    if !matches!(
        mode,
        ConversionSettingsApplicationMode::HtmlCleanup
            | ConversionSettingsApplicationMode::PromptAndHtmlCleanup
    ) || (settings.tables && settings.colors && settings.images)
    {
        return Ok(body.to_owned());
    }

    let dom = parse_fragment(
        RcDom::default(),
        Default::default(),
        QualName::new(None, ns!(html), local_name!(body)),
        Vec::new(),
        false,
    )
    .one(body.to_owned());
    let nodes = checked_nodes(&dom.document)?;
    // Flatten nested tables before moving their enclosing cells.
    for node in nodes.into_iter().rev() {
        transform_node(&dom, &node, &settings);
    }
    serialize_document(&dom)
}

pub(super) fn checked_nodes(root: &Handle) -> Result<Vec<Handle>, RenderingError> {
    let mut pending = vec![(root.clone(), 0)];
    let mut nodes = Vec::new();
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_DOM_DEPTH {
            return Err(RenderingError::HtmlNestingTooDeep);
        }
        pending.extend(
            node.children
                .borrow()
                .iter()
                .rev()
                .map(|child| (child.clone(), depth + 1)),
        );
        nodes.push(node);
    }
    Ok(nodes)
}

fn transform_node(dom: &RcDom, node: &Handle, settings: &ConversionSettings) {
    let tag = node.element_name().map(|name| name.to_ascii_lowercase());
    if tag.as_deref() == Some("img") && !settings.images {
        dom.remove_from_parent(node);
        return;
    }
    if !settings.colors {
        remove_colors(node);
    }
    if tag.as_deref() == Some("table") && !settings.tables {
        let replacement = flatten_table(dom, node);
        if let Some(parent) = node.get_parent() {
            let mut children = parent.children.borrow_mut();
            if let Some(index) = children.iter().position(|child| Rc::ptr_eq(child, node)) {
                replacement.parent.set(Some(Rc::downgrade(&parent)));
                node.parent.set(None);
                children[index] = replacement;
            }
        }
    }
}

fn remove_colors(node: &Handle) {
    let Element { ref attrs, .. } = node.data else {
        return;
    };
    let mut attrs = attrs.borrow_mut();
    let Some(index) = attrs
        .iter()
        .position(|attribute| attribute.name.local.as_ref() == "style")
    else {
        return;
    };
    let value = attrs[index].value.to_string();
    match styles::remove_colors(&value) {
        Some(value) => attrs[index].value = value.into(),
        None => {
            attrs.remove(index);
        }
    }
}

fn flatten_table(dom: &RcDom, table: &Handle) -> Handle {
    let output = element(dom, "div");
    let mut pending = table
        .children
        .borrow()
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        if is_element(&node, "caption") {
            let caption = element(dom, "p");
            if let (Element { attrs: source, .. }, Element { attrs: target, .. }) =
                (&node.data, &caption.data)
            {
                *target.borrow_mut() = source.borrow().clone();
            }
            dom.reparent_children(&node, &caption);
            dom.append(&output, NodeOrText::AppendNode(caption));
        } else if is_element(&node, "tr") {
            let row = element(dom, "div");
            let cells = node
                .children
                .borrow()
                .iter()
                .filter(|child| is_element(child, "td") || is_element(child, "th"))
                .cloned()
                .collect::<Vec<_>>();
            for (index, cell) in cells.iter().enumerate() {
                if index > 0 {
                    dom.append(&row, NodeOrText::AppendText(" ".into()));
                }
                if let Element { attrs, .. } = &cell.data {
                    if let Some(style) = attrs
                        .borrow()
                        .iter()
                        .find(|attribute| attribute.name.local.as_ref() == "style")
                    {
                        let wrapper = element(dom, "span");
                        if let Element { attrs: target, .. } = &wrapper.data {
                            target.borrow_mut().push(style.clone());
                        }
                        dom.reparent_children(cell, &wrapper);
                        dom.append(&row, NodeOrText::AppendNode(wrapper));
                        continue;
                    }
                }
                dom.reparent_children(cell, &row);
            }
            dom.append(&output, NodeOrText::AppendNode(row));
        } else if !is_element(&node, "table") {
            pending.extend(node.children.borrow().iter().rev().cloned());
        }
    }
    output
}

fn element(dom: &RcDom, name: &str) -> Handle {
    dom.create_element(
        QualName::new(None, ns!(html), name.into()),
        Vec::new(),
        Default::default(),
    )
}

fn is_element(node: &Handle, expected: &str) -> bool {
    node.element_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

pub(super) fn serialize_document(dom: &RcDom) -> Result<String, RenderingError> {
    checked_nodes(&dom.document)?;
    let children = dom.document.children.borrow();
    let mut bytes = Vec::new();
    // Fragment parsing creates an html root that is not part of the body.
    if children.len() == 1 && is_element(&children[0], "html") {
        for child in children[0].children.borrow().iter() {
            child
                .serialize(&mut bytes)
                .map_err(|_| RenderingError::PlainTextExtraction)?;
        }
    } else {
        dom.serialize(&mut bytes)
            .map_err(|_| RenderingError::PlainTextExtraction)?;
    }
    String::from_utf8(bytes).map_err(|_| RenderingError::PlainTextExtraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_excessive_nesting_before_cleanup_or_serialization() {
        let depth = MAX_DOM_DEPTH + 100;
        let input = format!("{}text{}", "<div>".repeat(depth), "</div>".repeat(depth));
        assert_eq!(
            apply(
                &input,
                ConversionSettings {
                    colors: false,
                    ..ConversionSettings::default()
                },
                ConversionSettingsApplicationMode::HtmlCleanup,
            ),
            Err(RenderingError::HtmlNestingTooDeep)
        );
    }
}
