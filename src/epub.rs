use super::xml::{Document, Node, ParsingOptions};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{self, PathBuf},
};
use zip::ZipArchive;

use super::Result;
use crate::error::{to_fnf_error, to_parse_error};

pub const EPUB_MIME_TYPE: &str = "application/epub+zip";

pub struct Epub {
    container: ZipArchive<File>,
    root_dir: String,
    pub file_path: path::PathBuf,
    pub metadata: Option<Metadata>,
    pub chapters: Vec<Chapter>,
    pub toc: Vec<(usize, String, String)>,
}

#[derive(Debug)]
pub struct Metadata {
    title: Option<String>,
    creator: Option<String>,
    language: Option<String>,
    date: Option<String>,
    identifier: Option<String>,
    description: Option<String>,
    publisher: Option<String>,
}

#[derive(Debug)]
pub struct Chapter {
    pub title: String,
    pub id: String,
    pub relative_path: String,
    pub text: Option<String>,
    // pub lines: Vec<(usize, usize)>,
}

impl Metadata {
    fn new(metadata_node: Node) -> Self {
        let mut metadata = Metadata {
            title: None,
            creator: None,
            language: None,
            identifier: None,
            publisher: None,
            date: None,
            description: None,
        };

        for child in metadata_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "title" => metadata.title = child.text().map(String::from),
                    "creator" => metadata.creator = child.text().map(String::from),
                    "language" => metadata.language = child.text().map(String::from),
                    "identifier" => metadata.identifier = child.text().map(String::from),
                    "publisher" => metadata.publisher = child.text().map(String::from),
                    "description" => metadata.description = child.text().map(String::from),
                    "date" => metadata.date = child.text().map(String::from),
                    _ => {}
                }
            }
        }

        return metadata;
    }
}

impl Chapter {
    fn new(id: &str, path: &str) -> Self {
        Chapter {
            title: id.to_string(),
            id: id.to_string(),
            relative_path: path.to_string(),
            text: None,
        }
    }

    // fn render(&mut self, n: Node, open: Attribute, close: Attribute) {
    //     self.state.set(open);
    //     self.attrs.push((self.text.len(), open, self.state));
    //     self.render_text(n);
    //     self.state.unset(open);
    //     self.attrs.push((self.text.len(), close, self.state));
    // }
    // fn render_text(&mut self, n: Node) {
    //     for child in n.children() {
    //         render(child, self);
    //     }
    // }
}

impl Epub {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file = File::open(&path).map_err(|_| to_fnf_error(path.display().to_string()))?;

        let mut epub = Epub {
            file_path: path,
            container: ZipArchive::new(file).map_err(|_| to_parse_error())?,
            root_dir: String::new(),
            chapters: Vec::new(),
            toc: Vec::new(),
            metadata: None,
        };
        // check mimetype
        if epub.get_raw_text("mimetype")? != EPUB_MIME_TYPE {
            return Err(to_parse_error());
        }

        epub.parse_content_opf()?;
        Ok(epub)
    }

    fn get_raw_text(&mut self, name: &str) -> Result<String> {
        let mut text = String::new();
        self.container
            .by_name(name)?
            .read_to_string(&mut text)
            .map_err(|_| to_parse_error())?;
        Ok(text)
    }

    fn parse_content_opf(&mut self) -> Result<()> {
        let xml = self.get_raw_text("META-INF/container.xml")?;
        let doc = Document::parse(&xml)?;
        let path = doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .ok_or_else(|| to_parse_error())?
            .attribute("full-path")
            .ok_or_else(|| to_parse_error())?;

        let xml = self.get_raw_text(path)?;
        let content_opf = Document::parse(&xml)?;

        self.root_dir = match path.rfind('/') {
            Some(n) => &path[..=n],
            None => "",
        }
        .to_string();

        let mut children = content_opf
            .root_element()
            .children()
            .filter(Node::is_element);

        let metadata_node = children.next().unwrap();
        let manifest_node = children.next().unwrap();
        let spine_node = children.next().unwrap();
        let version = content_opf
            .root_element()
            .attribute("version")
            .ok_or_else(|| to_parse_error())?;
        let mut toc_file_path: Option<&str> = None;

        // Parse Ebook Metadata
        self.metadata = Some(Metadata::new(metadata_node));

        // Parse ebook chapter links in order
        let mut manifest = HashMap::new();
        manifest_node
            .children()
            .filter(Node::is_element)
            .for_each(|n| {
                manifest.insert(n.attribute("id").unwrap(), n.attribute("href").unwrap());
                if version == "3.0" && n.attribute("properties") == Some("nav") {
                    toc_file_path = Some(n.attribute("href").unwrap());
                } else {
                    if n.attribute("media-type") == Some("application/x-dtbncx+xml") {
                        toc_file_path = Some(n.attribute("href").unwrap());
                    }
                }
            });

        // Parse TOC
        let mut nav: HashMap<String, (String, String)> = HashMap::new();
        if let Some(toc_path) = toc_file_path {
            let full_toc_path = format!("{}{}", self.root_dir, toc_path);
            self.parse_toc(version, &full_toc_path, &mut nav)?;
        }

        // Parse Ebook Chapters
        for (i, node) in spine_node.children().filter(Node::is_element).enumerate() {
            let id = node.attribute("idref").ok_or_else(to_parse_error)?;
            if let Some(path) = manifest.remove(id) {
                if let Some((exact_path, title)) = nav.remove(path) {
                    self.toc.push((i, title, exact_path));
                }
                self.chapters.push(Chapter::new(id, path));
            } else {
                return Err(to_parse_error().into());
            }
        }

        return Ok(());
    }

    fn parse_toc(
        &mut self,
        version: &str,
        toc_path: &String,
        nav: &mut HashMap<String, (String, String)>,
    ) -> Result<()> {
        let xml = self.get_raw_text(&toc_path)?;
        let opt = ParsingOptions {
            allow_dtd: true,
            nodes_limit: u32::MAX,
        };
        let doc = Document::parse_with_options(&xml, opt)?;

        if version == "3.0" {
            if let Some(ol) = doc
                .descendants()
                .find(|n| n.has_tag_name("nav"))
                .and_then(|n| n.children().find(|n| n.has_tag_name("ol")))
            {
                ol.descendants()
                    .filter(|n| n.has_tag_name("a"))
                    .for_each(|n| {
                        if let (Some(path), Some(text)) = (n.attribute("href"), n.text()) {
                            let np = path.split("#").next().unwrap();
                            nav.insert(np.to_string(), (path.to_string(), text.to_string()));
                        }
                    });
            }
        } else {
            if let Some(nav_map) = doc.descendants().find(|n| n.has_tag_name("navMap")) {
                nav_map
                    .descendants()
                    .filter(|n| n.has_tag_name("navPoint"))
                    .for_each(|n| {
                        if let (Some(path), Some(text)) = (
                            n.descendants()
                                .find(|n| n.has_tag_name("content"))
                                .and_then(|n| n.attribute("src")),
                            n.descendants()
                                .find(|n| n.has_tag_name("text"))
                                .and_then(|n| n.text()),
                        ) {
                            let np = path.split("#").next().unwrap();
                            nav.insert(np.to_string(), (path.to_string(), text.to_string()));
                        }
                    });
            }
        }
        Ok(())
    }

    pub fn read_chapter(&mut self, index: usize) -> Result<()> {
        let mut chapter = &self.chapters[index];
        let xml = self.get_raw_text(&format!("{}{}", self.root_dir, chapter.relative_path))?;
        let opt = ParsingOptions {
            allow_dtd: true,
            nodes_limit: u32::MAX,
        };
        let doc = Document::parse_with_options(&xml, opt)?;
        let body = doc.root_element().last_element_child().unwrap();

        println!("{:?}", doc.root_element());

        // render(body, &mut c);
        // if chapter.text.trim().is_empty() {
        //     return Ok(());
        // }
        // let relative = path.rsplit('/').next().unwrap();
        // self.links
        //     .insert(relative.to_string(), (self.chapters.len(), 0));
        // for (id, pos) in c.frag.drain(..) {
        //     let url = format!("{}#{}", relative, id);
        //     self.links.insert(url, (self.chapters.len(), pos));
        // }
        // for link in c.links.iter_mut() {
        //     if link.2.starts_with('#') {
        //         link.2.insert_str(0, relative);
        //     }
        // }
        Ok(())
    }
}

// fn render(n: Node, c: &mut Chapter) {
//     if n.is_text() {
//         let text = n.text().unwrap();
//         let content: Vec<_> = text.split_ascii_whitespace().collect();

//         if text.starts_with(char::is_whitespace) {
//             c.text.push(' ');
//         }
//         c.text.push_str(&content.join(" "));
//         if text.ends_with(char::is_whitespace) {
//             c.text.push(' ');
//         }
//         return;
//     }

//     if let Some(id) = n.attribute("id") {
//         c.frag.push((id.to_string(), c.text.len()));
//     }

//     match n.tag_name().name() {
//         "br" => c.text.push('\n'),
//         "hr" => c.text.push_str("\n* * *\n"),
//         "img" => c.text.push_str("\n[IMG]\n"),
//         "a" => {
//             match n.attribute("href") {
//                 // TODO open external urls in browser
//                 Some(url) if !url.starts_with("http") => {
//                     let start = c.text.len();
//                     c.render(n, Attribute::Underlined, Attribute::NoUnderline);
//                     c.links.push((start, c.text.len(), url.to_string()));
//                 }
//                 _ => c.render_text(n),
//             }
//         }
//         "em" => c.render(n, Attribute::Italic, Attribute::NoItalic),
//         "strong" => c.render(n, Attribute::Bold, Attribute::NormalIntensity),
//         "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
//             c.text.push('\n');
//             c.render(n, Attribute::Bold, Attribute::NormalIntensity);
//             c.text.push('\n');
//         }
//         "blockquote" | "div" | "p" | "tr" => {
//             // TODO compress newlines
//             c.text.push('\n');
//             c.render_text(n);
//             c.text.push('\n');
//         }
//         "li" => {
//             c.text.push_str("\n- ");
//             c.render_text(n);
//             c.text.push('\n');
//         }
//         "pre" => {
//             c.text.push_str("\n  ");
//             n
//                 .descendants()
//                 .filter(Node::is_text)
//                 .map(|n| n.text().unwrap().replace('\n', "\n  "))
//                 .for_each(|s| c.text.push_str(&s));
//             c.text.push('\n');
//         }
//         _ => c.render_text(n),
//     }
// }
