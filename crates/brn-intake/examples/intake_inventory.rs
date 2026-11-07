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

fn visit(node: &Node, assets: &[Value], elements: &mut Vec<Value>, blocks: &mut Vec<String>) {
    match node {
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
            elements.push(json!({"type":"list_item", "text":label}));
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
                let mut value = json!({"type":"image", "sha256":asset["sha256"],
                    "width":asset["width"], "height":asset["height"]});
                if asset["media_type"] == "image/png" {
                    let bytes: Vec<u8> = serde_json::from_value(asset["bytes"].clone()).unwrap();
                    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
                    decoder.set_transformations(
                        png::Transformations::EXPAND | png::Transformations::STRIP_16,
                    );
                    let mut reader = decoder.read_info().unwrap();
                    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
                    let info = reader.next_frame(&mut pixels).unwrap();
                    let pixels = &pixels[..info.buffer_size()];
                    // Canonical RGB for opaque RGB/RGBA fixtures, independent of PNG encoding.
                    let rgb = match info.color_type {
                        png::ColorType::Rgb => pixels.to_vec(),
                        png::ColorType::Rgba if pixels.chunks_exact(4).all(|p| p[3] == 255) => {
                            pixels
                                .chunks_exact(4)
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
    if matches!(
        node,
        Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
    ) {
        blocks.push(text(node));
    }
    for child in node.children().into_iter().flatten() {
        visit(child, assets, elements, blocks);
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
    visit(
        &ast,
        result["assets"].as_array().unwrap(),
        &mut elements,
        &mut blocks,
    );
    println!("{}", json!({"elements": elements, "blocks":blocks}));
}
