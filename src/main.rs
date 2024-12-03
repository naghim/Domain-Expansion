use reqwest::blocking::Client;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::rc::Rc;
use std::str::FromStr;

const HEADER: &str = r#"
▗▄▄▖▗▄▄▖     ▗▄▄▄   ▗▄▖ ▗▖  ▗▖ ▗▄▖ ▗▄▄▄▖▗▖  ▗▖    ▗▄▄▄▖▗▖  ▗▖▗▄▄▖  ▗▄▖ ▗▖  ▗▖ ▗▄▄▖▗▄▄▄▖ ▗▄▖ ▗▖  ▗▖    ▗▄▄▖▗▄▄▖ 
▐▌  ▐▌       ▐▌  █ ▐▌ ▐▌▐▛▚▞▜▌▐▌ ▐▌  █  ▐▛▚▖▐▌    ▐▌    ▝▚▞▘ ▐▌ ▐▌▐▌ ▐▌▐▛▚▖▐▌▐▌     █  ▐▌ ▐▌▐▛▚▖▐▌      ▐▌  ▐▌ 
▐▌  ▐▌       ▐▌  █ ▐▌ ▐▌▐▌  ▐▌▐▛▀▜▌  █  ▐▌ ▝▜▌    ▐▛▀▀▘  ▐▌  ▐▛▀▘ ▐▛▀▜▌▐▌ ▝▜▌ ▝▀▚▖  █  ▐▌ ▐▌▐▌ ▝▜▌      ▐▌  ▐▌ 
 ■   ■       ▐▙▄▄▀ ▝▚▄▞▘▐▌  ▐▌▐▌ ▐▌▗▄█▄▖▐▌  ▐▌    ▐▙▄▄▖▗▞▘▝▚▖▐▌   ▐▌ ▐▌▐▌  ▐▌▗▄▄▞▘▗▄█▄▖▝▚▄▞▘▐▌  ▐▌       ■   ■ 
 ■■■ ■■■                                                                                               ■■■ ■■■
"#;

#[derive(Deserialize)]
struct CrtShResponse {
    common_name: String,
}

#[derive(Clone)]
struct Node {
    name: String,
    children: Vec<Rc<RefCell<Node>>>,
}

struct Style {
    indent_prefix: String,
    t_prefix: String,
    last_prefix: Option<String>,
}

struct Options {
    style: Style,
    colored: bool,
    include_root: bool,
}
#[derive(Debug)]
enum NodeKind {
    Default,
    Last,
    Root,
}

impl FromStr for NodeKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(NodeKind::Default),
            "last" => Ok(NodeKind::Last),
            "root" => Ok(NodeKind::Root),
            _ => Err(()),
        }
    }
}

fn generate_node(
    node: &Rc<RefCell<Node>>,
    opts: &Options,
    kind: NodeKind,
    depth: usize,
) -> Vec<String> {
    let prefix = match kind {
        NodeKind::Root => "",
        NodeKind::Default => opts.style.t_prefix.as_str(),
        NodeKind::Last => opts
            .style
            .last_prefix
            .as_deref()
            .unwrap_or(&opts.style.t_prefix),
    };

    let indent: String = match kind {
        NodeKind::Root => "".to_string(),
        NodeKind::Last => " ".repeat(prefix.len()).to_string(),
        _ => format!(
            "{}{}",
            opts.style.indent_prefix,
            " ".repeat(prefix.len() - opts.style.indent_prefix.len())
        ),
    };

    let color = if opts.colored {
        PREFIX_TO_COLORS[depth % PREFIX_TO_COLORS.len()]
    } else {
        ""
    };

    let reset = if opts.colored { "\x1b[0m" } else { "" };

    let mut lines = vec![format!(
        "{}{}{}{}",
        color,
        prefix,
        node.borrow().name,
        reset
    )];
    let children = &node.borrow().children;

    for (i, child) in children.iter().enumerate() {
        let child_kind = if i == children.len() - 1 {
            NodeKind::Last
        } else {
            NodeKind::Default
        };

        for line in generate_node(child, opts, child_kind, depth + 1) {
            lines.push(format!("{}{}{}", color, indent, line));
        }
    }

    lines
}

fn generate(node: &Rc<RefCell<Node>>, opts: &Options) -> String {
    generate_node(
        node,
        opts,
        if opts.include_root {
            NodeKind::Last
        } else {
            NodeKind::Root
        },
        0,
    )
    .join("\n")
}

const PREFIX_TO_COLORS: [&str; 6] = [
    "\x1b[91m", "\x1b[92m", "\x1b[93m", "\x1b[94m", "\x1b[95m", "\x1b[96m",
];

fn crtsh(domain: &str, colored: bool) -> Result<String, Box<dyn Error>> {
    let url = format!("https://crt.sh/?q=%.{}&output=json", domain);
    let client = Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()?;

    let data: Vec<CrtShResponse> = response.json()?;
    let common_names: HashSet<String> = data.into_iter().map(|res| res.common_name).collect();

    Ok(create_tree(common_names, colored))
}

fn create_tree(common_names: HashSet<String>, colored: bool) -> String {
    let mut split_domains: Vec<Vec<String>> = common_names
        .into_iter()
        .map(|name| name.split('.').rev().map(String::from).collect())
        .collect();

    split_domains.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
    let mut name_to_node: HashMap<String, Rc<RefCell<Node>>> = HashMap::new();

    for domain in split_domains {
        for i in 0..domain.len() {
            let parent = domain[..i]
                .iter()
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join(".");
            let current = domain[..=i]
                .iter()
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join(".");
            if !name_to_node.contains_key(&current) {
                let node = Node {
                    name: current.clone(),
                    children: vec![],
                };
                name_to_node.insert(current.clone(), Rc::new(RefCell::new(node)));

                if let Some(actual_parent) = name_to_node.get(&parent) {
                    actual_parent
                        .borrow_mut()
                        .children
                        .push(name_to_node[&current].clone());
                }
            }
        }
    }

    let options = Options {
        style: Style {
            indent_prefix: "│".to_string(),
            t_prefix: "├─".to_string(),
            last_prefix: Some("└─".to_string()),
        },
        colored,
        include_root: false,
    };

    let mut lines = vec![];

    for node in name_to_node.values() {
        if node.borrow().name.matches('.').count() == 1 {
            lines.push(generate(node, &options));
        }
    }

    lines.join("\n")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let domain = args.get(1).expect("Domain name is required");
    let no_color = args.iter().any(|arg| arg == "--no-color");

    if !no_color {
        for (i, line) in HEADER.lines().enumerate() {
            println!(
                "{}{}{}",
                PREFIX_TO_COLORS[(i + 1) % PREFIX_TO_COLORS.len()],
                line,
                "\x1b[0m"
            );
        }
        println!();
    }

    match crtsh(domain, !no_color) {
        Ok(line) => {
            if !line.is_empty() {
                println!("{}", line);
            } else {
                println!("No data found");
            }
        }
        Err(_) => println!("Error: Unable to fetch data from crt.sh"),
    }
}
