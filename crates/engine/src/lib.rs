pub type NodeId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    StartTag { name: String },
    EndTag { name: String },
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Element(ElementData),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub root: NodeId,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutBox {
    pub node_id: NodeId,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    pub boxes: Vec<LayoutBox>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayCommand {
    FillRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: [u8; 4],
    },
    DrawText {
        x: u32,
        y: u32,
        text: String,
        color: [u8; 4],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayList {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub commands: Vec<DisplayCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSnippet {
    pub node_id: NodeId,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutput {
    pub tokens: Vec<Token>,
    pub document: Document,
    pub layout: LayoutTree,
    pub display_list: DisplayList,
    pub scripts: Vec<ScriptSnippet>,
}

pub fn render_document(input: &str, viewport_width: u32, viewport_height: u32) -> RenderOutput {
    let tokens = tokenize(input);
    let document = parse_document(&tokens);
    let layout = layout_document(&document, viewport_width, viewport_height);
    let display_list = build_display_list(&document, &layout, viewport_width, viewport_height);
    let scripts = collect_scripts(&document);

    RenderOutput {
        tokens,
        document,
        layout,
        display_list,
        scripts,
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let rest = &input[i..];
        if rest.starts_with("<!--") {
            if let Some(end) = rest.find("-->") {
                i += end + 3;
            } else {
                break;
            }
            continue;
        }

        if rest.starts_with('<') {
            let Some(close) = rest.find('>') else {
                break;
            };
            let inside = rest[1..close].trim();
            i += close + 1;

            if inside.is_empty() || inside.starts_with('!') {
                continue;
            }

            if let Some(stripped) = inside.strip_prefix('/') {
                let name = normalize_tag_name(stripped);
                if !name.is_empty() {
                    tokens.push(Token::EndTag { name });
                }
                continue;
            }

            let self_closing = inside.ends_with('/');
            let name = normalize_tag_name(inside);
            if name.is_empty() {
                continue;
            }

            tokens.push(Token::StartTag { name: name.clone() });

            if name == "script" {
                let script_rest = &input[i..];
                if let Some(script_end) = find_case_insensitive(script_rest, "</script>") {
                    let code = &script_rest[..script_end];
                    if !code.trim().is_empty() {
                        tokens.push(Token::Text(code.to_string()));
                    }
                    tokens.push(Token::EndTag {
                        name: "script".to_string(),
                    });
                    i += script_end + "</script>".len();
                }
                continue;
            }

            if self_closing || is_void_element(&name) {
                tokens.push(Token::EndTag { name });
            }

            continue;
        }

        if let Some(next_tag) = rest.find('<') {
            let text = &rest[..next_tag];
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                tokens.push(Token::Text(trimmed.to_string()));
            }
            i += next_tag;
        } else {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                tokens.push(Token::Text(trimmed.to_string()));
            }
            break;
        }
    }

    tokens
}

pub fn parse_document(tokens: &[Token]) -> Document {
    let mut nodes = vec![Node {
        parent: None,
        children: Vec::new(),
        kind: NodeKind::Element(ElementData {
            tag_name: "document".to_string(),
        }),
    }];

    let root = 0;
    let mut stack = vec![root];

    for token in tokens {
        match token {
            Token::StartTag { name } => {
                let parent = *stack.last().unwrap_or(&root);
                let node_id = nodes.len();
                nodes.push(Node {
                    parent: Some(parent),
                    children: Vec::new(),
                    kind: NodeKind::Element(ElementData {
                        tag_name: name.clone(),
                    }),
                });
                nodes[parent].children.push(node_id);
                if !is_void_element(name) {
                    stack.push(node_id);
                }
            }
            Token::EndTag { name } => {
                while stack.len() > 1 {
                    let node_id = *stack.last().unwrap_or(&root);
                    let should_pop = matches!(
                        &nodes[node_id].kind,
                        NodeKind::Element(el) if el.tag_name == *name
                    );
                    stack.pop();
                    if should_pop {
                        break;
                    }
                }
            }
            Token::Text(text) => {
                let parent = *stack.last().unwrap_or(&root);
                let node_id = nodes.len();
                nodes.push(Node {
                    parent: Some(parent),
                    children: Vec::new(),
                    kind: NodeKind::Text(text.clone()),
                });
                nodes[parent].children.push(node_id);
            }
        }
    }

    Document { root, nodes }
}

pub fn layout_document(
    document: &Document,
    viewport_width: u32,
    viewport_height: u32,
) -> LayoutTree {
    let mut boxes = Vec::new();
    let mut cursor_y = 8;

    for &child in &document.nodes[document.root].children {
        cursor_y = layout_node(
            document,
            child,
            0,
            cursor_y,
            viewport_width,
            viewport_height,
            &mut boxes,
        );
    }

    LayoutTree { boxes }
}

pub fn build_display_list(
    document: &Document,
    layout: &LayoutTree,
    viewport_width: u32,
    viewport_height: u32,
) -> DisplayList {
    let mut commands = Vec::new();

    commands.push(DisplayCommand::FillRect {
        x: 0,
        y: 0,
        width: viewport_width,
        height: viewport_height,
        color: [245, 245, 248, 255],
    });

    for layout_box in &layout.boxes {
        let color = color_for_node(document, layout_box.node_id);
        commands.push(DisplayCommand::FillRect {
            x: layout_box.x,
            y: layout_box.y,
            width: layout_box.width,
            height: layout_box.height,
            color,
        });

        if let Some(label) = label_for_node(document, layout_box.node_id) {
            commands.push(DisplayCommand::DrawText {
                x: layout_box.x.saturating_add(4),
                y: layout_box.y.saturating_add(4),
                text: label,
                color: [18, 24, 45, 255],
            });
        }
    }

    DisplayList {
        viewport_width,
        viewport_height,
        commands,
    }
}

fn collect_scripts(document: &Document) -> Vec<ScriptSnippet> {
    let mut snippets = Vec::new();
    for (node_id, node) in document.nodes.iter().enumerate() {
        let NodeKind::Element(el) = &node.kind else {
            continue;
        };

        if el.tag_name != "script" {
            continue;
        }

        let mut combined = String::new();
        for &child in &node.children {
            if let NodeKind::Text(text) = &document.nodes[child].kind {
                combined.push_str(text);
            }
        }

        if !combined.trim().is_empty() {
            snippets.push(ScriptSnippet {
                node_id,
                code: combined,
            });
        }
    }

    snippets
}

fn layout_node(
    document: &Document,
    node_id: NodeId,
    depth: u32,
    mut cursor_y: u32,
    viewport_width: u32,
    viewport_height: u32,
    boxes: &mut Vec<LayoutBox>,
) -> u32 {
    if cursor_y >= viewport_height {
        return cursor_y;
    }

    let node = &document.nodes[node_id];
    match &node.kind {
        NodeKind::Element(el) => {
            if el.tag_name == "script" {
                return cursor_y;
            }

            let x = 8 + depth.saturating_mul(12);
            let width = viewport_width.saturating_sub(x.saturating_add(8)).max(8);
            let height = element_height(el.tag_name.as_str());

            boxes.push(LayoutBox {
                node_id,
                x,
                y: cursor_y,
                width,
                height,
            });

            cursor_y = cursor_y.saturating_add(height).saturating_add(6);
            for &child in &node.children {
                cursor_y = layout_node(
                    document,
                    child,
                    depth.saturating_add(1),
                    cursor_y,
                    viewport_width,
                    viewport_height,
                    boxes,
                );
            }
        }
        NodeKind::Text(text) => {
            if !text.trim().is_empty() {
                let x = 12 + depth.saturating_mul(12);
                let width = viewport_width.saturating_sub(x.saturating_add(8)).max(8);
                boxes.push(LayoutBox {
                    node_id,
                    x,
                    y: cursor_y,
                    width,
                    height: 18,
                });
                cursor_y = cursor_y.saturating_add(24);
            }
        }
    }

    cursor_y
}

fn element_height(tag_name: &str) -> u32 {
    match tag_name {
        "html" => 26,
        "body" => 26,
        "h1" => 44,
        "h2" => 38,
        "p" => 26,
        "div" => 30,
        "section" => 34,
        _ => 24,
    }
}

fn color_for_node(document: &Document, node_id: NodeId) -> [u8; 4] {
    match &document.nodes[node_id].kind {
        NodeKind::Element(el) => match el.tag_name.as_str() {
            "html" => [233, 237, 248, 255],
            "body" => [236, 241, 251, 255],
            "header" | "footer" => [195, 212, 250, 255],
            "main" | "article" | "section" | "aside" => [206, 221, 250, 255],
            "nav" => [187, 206, 249, 255],
            "h1" => [169, 192, 248, 255],
            "h2" | "h3" => [179, 201, 248, 255],
            "p" | "li" | "td" | "th" => [217, 228, 251, 255],
            _ => [210, 224, 250, 255],
        },
        NodeKind::Text(_) => [244, 246, 252, 255],
    }
}

fn label_for_node(document: &Document, node_id: NodeId) -> Option<String> {
    match &document.nodes[node_id].kind {
        NodeKind::Element(el) => Some(format!("<{}>", el.tag_name)),
        NodeKind::Text(text) => {
            let condensed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if condensed.is_empty() {
                None
            } else {
                Some(truncate_text(&condensed, 64))
            }
        }
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let suffix = "...";
    let keep = max_chars.saturating_sub(suffix.len());
    let mut out = String::with_capacity(max_chars + suffix.len());
    for ch in text.chars().take(keep) {
        out.push(ch);
    }
    out.push_str(suffix);
    out
}

fn normalize_tag_name(raw: &str) -> String {
    raw.trim_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_void_element(name: &str) -> bool {
    matches!(name, "br" | "img" | "meta" | "link" | "hr" | "input")
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_html() {
        let input = "<html><body><h1>Hello</h1><p>world</p></body></html>";
        let tokens = tokenize(input);

        assert!(tokens.contains(&Token::StartTag {
            name: "html".to_string()
        }));
        assert!(tokens.contains(&Token::StartTag {
            name: "h1".to_string()
        }));
        assert!(tokens.contains(&Token::Text("Hello".to_string())));
        assert!(tokens.contains(&Token::Text("world".to_string())));
    }

    #[test]
    fn builds_dom_shape() {
        let input = "<html><body><h1>Hello</h1><p>Body</p></body></html>";
        let doc = parse_document(&tokenize(input));

        let root_children = &doc.nodes[doc.root].children;
        assert_eq!(root_children.len(), 1);
        let html = root_children[0];

        let NodeKind::Element(el) = &doc.nodes[html].kind else {
            panic!("expected html element");
        };
        assert_eq!(el.tag_name, "html");

        let body = doc.nodes[html].children[0];
        let NodeKind::Element(body_el) = &doc.nodes[body].kind else {
            panic!("expected body element");
        };
        assert_eq!(body_el.tag_name, "body");
    }

    #[test]
    fn layout_and_display_list_are_stable() {
        let input = "<html><body><h1>Title</h1><p>Copy</p></body></html>";
        let output = render_document(input, 640, 360);

        assert!(output.layout.boxes.len() >= 4);
        assert!(matches!(
            output.display_list.commands.first(),
            Some(DisplayCommand::FillRect { x: 0, y: 0, .. })
        ));

        let mut ys: Vec<u32> = output.layout.boxes.iter().map(|b| b.y).collect();
        ys.sort_unstable();
        assert_eq!(
            ys,
            output.layout.boxes.iter().map(|b| b.y).collect::<Vec<_>>()
        );
    }

    #[test]
    fn display_list_includes_text_commands() {
        let input = "<html><body><h1>Hello</h1><p>Visible text</p></body></html>";
        let output = render_document(input, 640, 360);

        assert!(
            output
                .display_list
                .commands
                .iter()
                .any(|cmd| matches!(cmd, DisplayCommand::DrawText { .. }))
        );
    }

    #[test]
    fn script_extraction_is_deterministic() {
        let input = "<html><body><script>window.answer = 42;</script></body></html>";
        let output = render_document(input, 800, 600);

        assert_eq!(output.scripts.len(), 1);
        assert_eq!(output.scripts[0].code, "window.answer = 42;");
    }
}
