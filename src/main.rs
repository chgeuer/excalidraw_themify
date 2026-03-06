use std::env;
use std::fs;
use std::process;
use std::thread;
use std::time::Duration;
use regex::Regex;

/// Default Excalidraw colors that get themed
const DEFAULT_COLORS: &[&str] = &["#1e1e1e", "#ffffff"];

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: excalidraw_themify <input_file.svg>");
        eprintln!("       excalidraw_themify --watch-clipboard");
        process::exit(1);
    }

    if args[1] == "--watch-clipboard" {
        watch_clipboard();
    } else {
        transform_file(&args[1]);
    }
}

fn transform_file(input_path: &str) {
    let content = fs::read_to_string(input_path).expect("Could not read file");
    let processed = transform_svg(&content);

    let output_path = if input_path.ends_with(".svg") {
        input_path.replace(".svg", ".theme.svg")
    } else {
        format!("{}.theme.svg", input_path)
    };

    fs::write(&output_path, processed).expect("Could not write file");
    println!("Success! Created: {}", output_path);
}

/// Returns true if the content looks like an Excalidraw SVG that hasn't been themed yet.
fn is_excalidraw_svg(content: &str) -> bool {
    content.contains("<!-- svg-source:excalidraw -->") && !content.contains("var(--stroke)")
}

fn watch_clipboard() {
    let mut clipboard = arboard::Clipboard::new().expect("Could not access clipboard");
    let mut last_content = String::new();

    eprintln!("Watching clipboard for Excalidraw SVGs... (Ctrl+C to stop)");

    loop {
        if let Ok(text) = clipboard.get_text() {
            if text != last_content && is_excalidraw_svg(&text) {
                let themed = transform_svg(&text);
                match clipboard.set_text(&themed) {
                    Ok(()) => eprintln!("Themed an Excalidraw SVG in clipboard."),
                    Err(e) => eprintln!("Failed to update clipboard: {}", e),
                }
                last_content = themed;
            } else {
                last_content = text;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

/// Transform an Excalidraw SVG to support light/dark themes.
fn transform_svg(content: &str) -> String {
    // 1. Prepare the CSS Style block
    let style_block = r#"
    <style>
      :root {
        --stroke: #1e1e1e;
        --fill: #ffffff;
      }
      @media (prefers-color-scheme: dark) {
        :root {
          --stroke: #f0f0f0;
          --fill: #2c2c2c;
        }
      }
      text { font-family: Excalifont, 'Segoe UI', Roboto, sans-serif !important; }
    </style>"#;

    // 2. Inject Style Block into <defs>
    let mut processed = if content.contains("<defs>") {
        content.replace("<defs>", &format!("<defs>{}", style_block))
    } else {
        let re_svg_tag = Regex::new(r"(<svg[^>]*>)").unwrap();
        re_svg_tag.replace(content, format!("$1{}", style_block)).to_string()
    };

    // 3. Remove the hardcoded background rectangle
    let re_bg = Regex::new(r##"<rect x="0" y="0" [^>]*fill="#ffffff"[^>]*></rect>"##).unwrap();
    processed = re_bg.replace_all(&processed, "").to_string();

    // 4. Replace Hex codes with CSS Variables, but skip groups that contain
    //    custom (non-default) colors to preserve readable contrast.
    let skip_ranges = find_custom_color_group_ranges(&processed);
    processed = apply_color_replacements(&processed, &skip_ranges);

    processed
}

/// Check if a color value is a non-default (custom) color
fn is_custom_color(value: &str) -> bool {
    let v = value.to_lowercase();
    !DEFAULT_COLORS.contains(&v.as_str()) && v != "none" && v != "transparent"
}

/// Check if any descendant of the node has a custom fill or stroke color
fn subtree_has_custom_colors(node: &roxmltree::Node) -> bool {
    node.descendants().any(|desc| {
        desc.is_element()
            && ["fill", "stroke"]
                .iter()
                .any(|&attr| desc.attribute(attr).is_some_and(|v| is_custom_color(v)))
    })
}

/// Find byte ranges of top-level `<g>` groups that contain custom colors.
/// Also includes the next sibling `<g>` group after each custom-colored group,
/// since Excalidraw places bound text in a separate sibling group.
fn find_custom_color_group_ranges(svg_content: &str) -> Vec<(usize, usize)> {
    let doc = match roxmltree::Document::parse(svg_content) {
        Ok(doc) => doc,
        Err(_) => return Vec::new(), // fall back to replacing everything
    };

    let root = doc.root_element();
    let groups: Vec<_> = root
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "g")
        .collect();

    let mut skip_ranges = Vec::new();
    let mut skip_next = false;

    for g in &groups {
        if subtree_has_custom_colors(g) {
            let r = g.range();
            skip_ranges.push((r.start, r.end));
            skip_next = true;
        } else if skip_next {
            // The sibling text group right after a custom-colored shape group
            let r = g.range();
            skip_ranges.push((r.start, r.end));
            skip_next = false;
        } else {
            skip_next = false;
        }
    }

    skip_ranges
}

/// Apply color replacements, skipping byte positions within skip_ranges
fn apply_color_replacements(input: &str, skip_ranges: &[(usize, usize)]) -> String {
    const REPLACEMENTS: &[(&str, &str)] = &[
        ("fill=\"#1e1e1e\"", "fill=\"var(--stroke)\""),
        ("stroke=\"#1e1e1e\"", "stroke=\"var(--stroke)\""),
        ("fill=\"#ffffff\"", "fill=\"var(--fill)\""),
        ("stroke=\"#ffffff\"", "stroke=\"var(--fill)\""),
    ];

    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;

    while pos < bytes.len() {
        let in_skip = skip_ranges.iter().any(|&(s, e)| pos >= s && pos < e);

        if !in_skip {
            if let Some(&(pattern, replacement)) = REPLACEMENTS
                .iter()
                .find(|&&(pat, _)| bytes[pos..].starts_with(pat.as_bytes()))
            {
                result.push_str(replacement);
                pos += pattern.len();
                continue;
            }
        }

        let c = input[pos..].chars().next().unwrap();
        result.push(c);
        pos += c.len_utf8();
    }

    result
}
