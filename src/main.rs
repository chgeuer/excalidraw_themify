use std::env;
use std::fs;
use std::process;
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: themeify <input_file.svg>");
        process::exit(1);
    }

    let input_path = &args[1];
    let content = fs::read_to_string(input_path).expect("Could not read file");

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
        re_svg_tag.replace(&content, format!("$1{}", style_block)).to_string()
    };

    // 3. Remove the hardcoded background rectangle
    // Note the double ## here to handle the internal # symbols
    let re_bg = Regex::new(r##"<rect x="0" y="0" [^>]*fill="#ffffff"[^>]*></rect>"##).unwrap();
    processed = re_bg.replace_all(&processed, "").to_string();

    // 4. Replace Hex codes with CSS Variables
    let replacements = [
        (r##"fill="#1e1e1e""##, r#"fill="var(--stroke)""#),
        (r##"stroke="#1e1e1e""##, r#"stroke="var(--stroke)""#),
        (r##"fill="#ffffff""##, r#"fill="var(--fill)""#),
        (r##"stroke="#ffffff""##, r#"stroke="var(--fill)""#),
    ];

    for (pattern, replacement) in replacements {
        let re = Regex::new(pattern).unwrap();
        processed = re.replace_all(&processed, replacement).to_string();
    }

    // 5. Write the output
    let output_path = if input_path.ends_with(".svg") {
        input_path.replace(".svg", ".theme.svg")
    } else {
        format!("{}.theme.svg", input_path)
    };

    fs::write(&output_path, processed).expect("Could not write file");

    println!("Success! Created: {}", output_path);
}
