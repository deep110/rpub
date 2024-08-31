fn render(n: Node, c: &mut Chapter) {
    if n.is_text() {
        let text = n.text().unwrap();
        let content: Vec<_> = text.split_ascii_whitespace().collect();

        if text.starts_with(char::is_whitespace) {
            c.text.push(' ');
        }
        c.text.push_str(&content.join(" "));
        if text.ends_with(char::is_whitespace) {
            c.text.push(' ');
        }
        return;
    }

    if let Some(id) = n.attribute("id") {
        c.frag.push((id.to_string(), c.text.len()));
    }

    match n.tag_name().name() {
        "br" => c.text.push('\n'),
        "hr" => c.text.push_str("\n* * *\n"),
        "img" => c.text.push_str("\n[IMG]\n"),
        "a" => {
            match n.attribute("href") {
                // TODO open external urls in browser
                Some(url) if !url.starts_with("http") => {
                    let start = c.text.len();
                    c.render(n, Attribute::Underlined, Attribute::NoUnderline);
                    c.links.push((start, c.text.len(), url.to_string()));
                }
                _ => c.render_text(n),
            }
        }
        "em" => c.render(n, Attribute::Italic, Attribute::NoItalic),
        "strong" => c.render(n, Attribute::Bold, Attribute::NormalIntensity),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            c.text.push('\n');
            c.render(n, Attribute::Bold, Attribute::NormalIntensity);
            c.text.push('\n');
        }
        "blockquote" | "div" | "p" | "tr" => {
            // TODO compress newlines
            c.text.push('\n');
            c.render_text(n);
            c.text.push('\n');
        }
        "li" => {
            c.text.push_str("\n- ");
            c.render_text(n);
            c.text.push('\n');
        }
        "pre" => {
            c.text.push_str("\n  ");
            n.descendants()
                .filter(Node::is_text)
                .map(|n| n.text().unwrap().replace('\n', "\n  "))
                .for_each(|s| c.text.push_str(&s));
            c.text.push('\n');
        }
        _ => c.render_text(n),
    }
}