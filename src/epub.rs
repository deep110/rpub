use roxmltree::{Document, Node, ParsingOptions};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{self, PathBuf},
    sync::OnceLock,
};
use zip::ZipArchive;

use crate::error::{to_fnf_error, to_parse_error};
use super::Result;

static MIME_TYPE: OnceLock<String> = OnceLock::new();

fn epub_mimetype() -> &'static String {
    MIME_TYPE.get_or_init(|| String::from("application/epub+zip"))
}

pub struct Epub {
    container: ZipArchive<File>,
    // content_opf: Option<String>,
    pub file_path: path::PathBuf,
    pub chapters: Vec<Chapter>,
    pub links: HashMap<String, (usize, usize)>,
    pub metadata: Option<Metadata>,
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

pub struct Chapter {
    pub title: String,
    // single string for search
    pub text: String,
    pub lines: Vec<(usize, usize)>,
    // crossterm gives us a bitset but doesn't let us diff it, so store the state transition
    // pub attrs: Vec<(usize, Attribute, Attributes)>,
    pub links: Vec<(usize, usize, String)>,
    frag: Vec<(String, usize)>,
    // state: Attributes,
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


impl Epub {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file =
            File::open(&path).map_err(|_| to_fnf_error(path.display().to_string()))?;

        let mut epub = Epub {
            file_path: path,
            container: ZipArchive::new(file).map_err(|_| to_parse_error())?,
            // content_opf: None,
            chapters: Vec::new(),
            links: HashMap::new(),
            metadata: None,
        };
        // check mimetype
        if epub.get_raw_text("mimetype")? != *epub_mimetype() {
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

    fn parse_content_opf(&mut self) -> Result<()>{
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

        let version = content_opf.root_element().attribute("version");
        println!("{:?}", version);
        // TODO: add version check

        // let mut manifest = HashMap::new();
        // let mut nav = HashMap::new();
        let mut children = content_opf.root_element().children().filter(Node::is_element);
        let metadata_node = children.next().unwrap();
        let manifest_node = children.next().unwrap();
        let spine_node = children.next().unwrap();

        self.metadata = Some(Metadata::new(metadata_node));

        
        return Ok(());
    }


    fn parse_chapters(&mut self) -> Result<()> {
        Ok(())

        // zip expects unix path even on windows
        // self.rootdir = match path.rfind('/') {
        //     Some(n) => &path[..=n],
        //     None => "",
        // }
        // .to_string();
        // let mut manifest = HashMap::new();
        // let mut nav = HashMap::new();
        // let mut children = doc.root_element().children().filter(Node::is_element);
        // let meta_node = children.next().unwrap();
        // let manifest_node = children.next().unwrap();
        // let spine_node = children.next().unwrap();

        // meta_node.children().filter(Node::is_element).for_each(|n| {
        //     let name = n.tag_name().name();
        //     let text = n.text();
        //     if text.is_some() && name != "meta" {
        //         self.meta
        //             .push_str(&format!("{}: {}\n", name, text.unwrap()));
        //     }
        // });
        // manifest_node
        //     .children()
        //     .filter(Node::is_element)
        //     .for_each(|n| {
        //         manifest.insert(n.attribute("id").unwrap(), n.attribute("href").unwrap());
        //     });
        // if doc.root_element().attribute("version") == Some("3.0") {
        //     let path = manifest_node
        //         .children()
        //         .find(|n| n.attribute("properties") == Some("nav"))
        //         .unwrap()
        //         .attribute("href")
        //         .unwrap();
        //     let xml = self.get_text(&format!("{}{}", self.rootdir, path));
        //     let doc = Document::parse(&xml).unwrap();
        //     epub3(doc, &mut nav);
        // } else {
        //     let id = spine_node.attribute("toc").unwrap_or("ncx");
        //     let path = manifest.get(id).unwrap();
        //     let xml = self.get_text(&format!("{}{}", self.rootdir, path));
        //     let doc = Document::parse(&xml).unwrap();
        //     epub2(doc, &mut nav);
        // }
        // spine_node
        //     .children()
        //     .filter(Node::is_element)
        //     .enumerate()
        //     .map(|(i, n)| {
        //         let id = n.attribute("idref").unwrap();
        //         let path = manifest.remove(id).unwrap();
        //         let label = nav.remove(path).unwrap_or_else(|| i.to_string());
        //         (label, path.to_string())
        //     })
        //     .collect()
    }

    // fn get_chapters(&mut self, spine: Vec<(String, String)>) {
    //     for (title, path) in spine {
    //         // https://github.com/RazrFalcon/roxmltree/issues/12
    //         // UnknownEntityReference for HTML entities
    //         let xml = self.get_text(&format!("{}{}", self.rootdir, path));
    //         let opt = ParsingOptions { allow_dtd: true };
    //         let doc = Document::parse_with_options(&xml, opt).unwrap();
    //         let body = doc.root_element().last_element_child().unwrap();
    //         let state = Attributes::default();
    //         let mut c = Chapter {
    //             title,
    //             text: String::new(),
    //             lines: Vec::new(),
    //             attrs: vec![(0, Attribute::Reset, state)],
    //             state,
    //             links: Vec::new(),
    //             frag: Vec::new(),
    //         };
    //         render(body, &mut c);
    //         if c.text.trim().is_empty() {
    //             continue;
    //         }
    //         let relative = path.rsplit('/').next().unwrap();
    //         self.links
    //             .insert(relative.to_string(), (self.chapters.len(), 0));
    //         for (id, pos) in c.frag.drain(..) {
    //             let url = format!("{}#{}", relative, id);
    //             self.links.insert(url, (self.chapters.len(), pos));
    //         }
    //         for link in c.links.iter_mut() {
    //             if link.2.starts_with('#') {
    //                 link.2.insert_str(0, relative);
    //             }
    //         }
    //         self.chapters.push(c);
    //     }
    // }

}

// impl Chapter {
//     fn render(&mut self, n: Node, open: Attribute, close: Attribute) {
//         self.state.set(open);
//         self.attrs.push((self.text.len(), open, self.state));
//         self.render_text(n);
//         self.state.unset(open);
//         self.attrs.push((self.text.len(), close, self.state));
//     }
//     fn render_text(&mut self, n: Node) {
//         for child in n.children() {
//             render(child, self);
//         }
//     }
// }

fn epub2(doc: Document, nav: &mut HashMap<String, String>) {
    doc.descendants()
        .find(|n| n.has_tag_name("navMap"))
        .unwrap()
        .descendants()
        .filter(|n| n.has_tag_name("navPoint"))
        .for_each(|n| {
            let path = n
                .descendants()
                .find(|n| n.has_tag_name("content"))
                .unwrap()
                .attribute("src")
                .unwrap()
                .split('#')
                .next()
                .unwrap()
                .to_string();
            let text = n
                .descendants()
                .find(|n| n.has_tag_name("text"))
                .unwrap()
                .text()
                .unwrap()
                .to_string();
            // TODO subsections
            nav.entry(path).or_insert(text);
        });
}

fn epub3(doc: Document, nav: &mut HashMap<String, String>) {
    doc.descendants()
        .find(|n| n.has_tag_name("nav"))
        .unwrap()
        .children()
        .find(|n| n.has_tag_name("ol"))
        .unwrap()
        .descendants()
        .filter(|n| n.has_tag_name("a"))
        .for_each(|n| {
            let path = n
                .attribute("href")
                .unwrap()
                .split('#')
                .next()
                .unwrap()
                .to_string();
            let text = n
                .descendants()
                .filter(Node::is_text)
                .map(|n| n.text().unwrap())
                .collect();
            nav.insert(path, text);
        });
}
