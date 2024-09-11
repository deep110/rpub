use super::xml::{Document, Node};
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
    pub relative_path: String,
    pub text: String,
    ids: Vec<(String, usize)>,
    is_parsed: bool,
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
    fn new(path: &str) -> Self {
        Chapter {
            relative_path: path.to_string(),
            text: String::new(),
            ids: Vec::new(),
            is_parsed: false,
        }
    }

    fn parse_children(&mut self, node: Node) {
        for child in node.children() {
            self.parse(child);
        }
    }

    fn parse(&mut self, n: Node) {
        if n.is_text() {
            let text = n.text().unwrap();
            let content: Vec<_> = text.split_ascii_whitespace().collect();

            if text.starts_with(char::is_whitespace) {
                self.text.push(' ');
            }
            self.text.push_str(&content.join(" "));
            if text.ends_with(char::is_whitespace) {
                self.text.push(' ');
            }
            return;
        }

        if let Some(id) = n.attribute("id") {
            self.ids.push((id.to_string(), self.text.len()));
        }

        match n.tag_name().name() {
            "br" => self.text.push('\n'),
            "hr" => self.text.push_str("\n* * *\n"),
            "img" | "image" => self.text.push_str("\n[IMAGE]\n"),
            "a" => {
                match n.attribute("href") {
                    // TODO open external urls in browser
                    Some(url) if !url.starts_with("http") => {
                        // let start = c.text.len();
                        self.text.push_str(&termion::style::Underline.to_string());
                        self.parse_children(n);
                        self.text.push_str(&termion::style::NoUnderline.to_string());
                        // c.links.push((start, c.text.len(), url.to_string()));
                    }
                    _ => self.parse_children(n),
                }
            }
            "em" => {
                self.text.push_str(&termion::style::Italic.to_string());
                self.parse_children(n);
                self.text.push_str(&termion::style::NoItalic.to_string());
            }
            "strong" => {
                self.text.push_str(&termion::style::Bold.to_string());
                self.parse_children(n);
                self.text.push_str(&termion::style::Reset.to_string());
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.text.push('\n');
                self.text.push_str(&termion::style::Bold.to_string());
                self.parse_children(n);
                self.text.push_str(&termion::style::Reset.to_string());
                self.text.push('\n');
            }
            "blockquote" | "div" | "p" | "tr" => {
                // TODO compress newlines
                self.text.push('\n');
                self.parse_children(n);
                self.text.push('\n');
            }
            "li" => {
                self.text.push_str("\n- ");
                self.parse_children(n);
                self.text.push('\n');
            }
            "pre" => {
                self.text.push_str("\n  ");
                n.descendants()
                    .filter(Node::is_text)
                    .map(|n| n.text().unwrap().replace('\n', "\n  "))
                    .for_each(|s| self.text.push_str(&s));
                self.text.push('\n');
            }
            _ => self.parse_children(n),
        }
    }
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
            .req_attribute("full-path")?;

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
        let version = content_opf.root_element().req_attribute("version")?;

        // Parse Ebook Metadata
        self.metadata = Some(Metadata::new(metadata_node));

        // Parse ebook chapter links in order
        let mut manifest: HashMap<&str, &str> = HashMap::new();
        let mut toc_file_path: Option<&str> = None;
        for n in manifest_node.children().filter(Node::is_element) {
            manifest.insert(n.req_attribute("id")?, n.req_attribute("href")?);
            if version == "3.0" && n.attribute("properties") == Some("nav") {
                toc_file_path = n.attribute("href");
            } else if n.attribute("media-type") == Some("application/x-dtbncx+xml") {
                toc_file_path = n.attribute("href");
            }
        }

        // Parse TOC
        let mut nav: HashMap<String, (String, String)> = HashMap::new();
        if let Some(toc_path) = toc_file_path {
            let full_toc_path = format!("{}{}", self.root_dir, toc_path);
            self.parse_toc(version, &full_toc_path, &mut nav)?;
        }

        // Parse Ebook Chapters
        for (i, node) in spine_node.children().filter(Node::is_element).enumerate() {
            let id = node.req_attribute("idref")?;
            if let Some(path) = manifest.remove(id) {
                if let Some((exact_path, title)) = nav.remove(path) {
                    self.toc.push((i, title, exact_path));
                }
                self.chapters.push(Chapter::new(path));
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
        let doc = Document::parse(&xml)?;

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

    pub fn read_chapter(&mut self, index: usize) -> Result<&String> {
        if self.chapters[index].is_parsed {
            return Ok(&self.chapters[index].text);
        }

        let relative_path = self.chapters[index].relative_path.clone();
        let xml = self.get_raw_text(&format!("{}{}", &self.root_dir, relative_path))?;

        let doc = Document::parse(&xml)?;
        let body = doc.root_element().last_element_child().unwrap();

        self.chapters[index].parse(body);
        self.chapters[index].is_parsed = true;

        Ok(&self.chapters[index].text)
    }
}
