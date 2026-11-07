//! Independent GFM inventory for the offline scorer. Never calls a converter.
use markdown::{ParseOptions, mdast::Node};
use serde_json::{Value, json};
use std::io::Read;

fn text(node: &Node) -> String {
    match node {
        Node::Text(t) => t.value.clone(),
        Node::InlineCode(c) => c.value.clone(),
        Node::Break(_) => " ".into(),
        _ => node.children().into_iter().flatten().map(text).collect(),
    }
}

fn visit(
    node: &Node,
    assets: &[Value],
    elements: &mut Vec<Value>,
    blocks: &mut Vec<String>,
    context: &[String],
    lists: &[(bool, u32, usize)],
    block_positions: &mut Vec<usize>,
) {
    let first = elements.len();
    match node {
        Node::Definition(d) => elements.push(
            json!({"type":"definition", "identifier":d.identifier, "url":d.url, "title":d.title}),
        ),
        Node::Html(h) => elements.push(json!({"type":"html", "text":h.value})),
        Node::Link(l) => {
            elements.push(json!({"type":"link", "text":text(node), "url":l.url, "title":l.title}))
        }
        Node::LinkReference(l) => elements
            .push(json!({"type":"link_reference", "text":text(node), "identifier":l.identifier})),
        Node::Blockquote(_) => elements.push(json!({"type":"blockquote", "text":text(node)})),
        Node::Code(c) => elements.push(json!({"type":"code", "text":c.value})),
        Node::Heading(h) => {
            elements.push(json!({"type":"heading", "level":h.depth, "text":text(node)}))
        }
        Node::ListItem(_) => {
            // Exclude nested lists from the parent's label, but inventory them separately.
            let label = node
                .children()
                .into_iter()
                .flatten()
                .filter(|n| !matches!(n, Node::List(_)))
                .map(text)
                .collect::<Vec<_>>()
                .join(" ");
            let (ordered, start, index) = lists.last().copied().unwrap();
            elements.push(json!({"type":"list_item", "text":label,
                "ordered":ordered, "depth":lists.len(), "ordinal":if ordered {Some(start + index as u32)} else {None}}));
        }
        Node::Table(table) => {
            let cells: Vec<Vec<String>> = table
                .children
                .iter()
                .map(|r| r.children().into_iter().flatten().map(text).collect())
                .collect();
            elements.push(json!({"type":"table", "rows":cells.len(),
                "columns":cells.first().map_or(0, Vec::len), "cells":cells}));
        }
        Node::Image(image) => {
            if let Some(asset) = assets
                .iter()
                .find(|a| a["name"].as_str() == Some(&image.url))
            {
                let bytes: Vec<u8> = serde_json::from_value(asset["bytes"].clone()).unwrap();
                let (width, height) = brn_intake::validate_document_image(
                    &bytes,
                    asset["media_type"].as_str().unwrap(),
                    &brn_intake::IntakeLimits::default(),
                )
                .unwrap();
                let mut value = json!({"type":"image", "sha256":brn_intake::digest(&bytes),
                    "width":width, "height":height});
                if asset["media_type"] == "image/png" {
                    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
                    decoder.set_transformations(
                        png::Transformations::EXPAND | png::Transformations::STRIP_16,
                    );
                    let mut reader = decoder.read_info().unwrap();
                    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
                    let info = reader.next_frame(&mut pixels).unwrap();
                    value["width"] = json!(info.width);
                    value["height"] = json!(info.height);
                    let pixels = &pixels[..info.buffer_size()];
                    // Canonical RGB for opaque RGB/RGBA fixtures, independent of PNG encoding.
                    let rgb = match info.color_type {
                        png::ColorType::Rgb => pixels.to_vec(),
                        png::ColorType::Rgba
                            if pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255) =>
                        {
                            pixels
                                .as_chunks::<4>()
                                .0
                                .iter()
                                .flat_map(|p| p[..3].iter().copied())
                                .collect()
                        }
                        _ => Vec::new(),
                    };
                    if !rgb.is_empty() {
                        value["pixels_sha256"] = json!(brn_intake::hex(&brn_intake::digest(&rgb)));
                    }
                }
                elements.push(value);
            }
        }
        _ => {}
    }
    for value in &mut elements[first..] {
        value["context"] = json!(context);
        value["position"] = json!(node.position().map_or(0, |p| p.start.offset));
    }
    if matches!(
        node,
        Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
    ) {
        blocks.push(text(node));
        block_positions.push(node.position().map_or(0, |p| p.start.offset));
    }
    let mut context = context.to_vec();
    match node {
        Node::Blockquote(_) => context.push("blockquote".into()),
        Node::List(l) => context.push(format!("list:{}:{}", l.ordered, l.start.unwrap_or(1))),
        Node::ListItem(_) => context.push(format!("item:{}", text(node))),
        Node::TableCell(_) => context.push(format!("cell:{}", text(node))),
        _ => {}
    }
    for (index, child) in node.children().into_iter().flatten().enumerate() {
        let mut lists = lists.to_vec();
        if let Node::List(list) = node {
            lists.push((list.ordered, list.start.unwrap_or(1), index));
        }
        visit(
            child,
            assets,
            elements,
            blocks,
            &context,
            &lists,
            block_positions,
        );
    }
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let result: Value = serde_json::from_str(&input).unwrap();
    let ast =
        markdown::to_mdast(result["markdown"].as_str().unwrap(), &ParseOptions::gfm()).unwrap();
    let mut elements = Vec::new();
    let mut blocks = Vec::new();
    let mut block_positions = Vec::new();
    visit(
        &ast,
        result["assets"].as_array().unwrap(),
        &mut elements,
        &mut blocks,
        &[],
        &[],
        &mut block_positions,
    );
    println!(
        "{}",
        json!({"elements": elements, "blocks":blocks, "block_positions":block_positions})
    );
}
