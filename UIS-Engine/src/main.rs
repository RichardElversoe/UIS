/// NOT YET ALPHA RELEASE!!!
/// To everyone reading this file - Keep in mind that this is extremely early stages. Things WILL change, and drastic modifications will occur.
/// RUN IT AT YOUR OWN RISK

use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};

#[macro_export]
macro_rules! timer {
    ($label:expr, $code:block) => {{
        use std::time::Instant;
        let start = Instant::now();
        let result = $code;
        let elapsed = start.elapsed();
        println!("{} took: {:.2?}", $label, elapsed);
        result
    }};
}

struct UISFile {
    name: String,
    data: String,
}

#[derive(Debug)]
struct UIS {
    components: IndexMap<String, Component>, //Index map - Order is important here!
    properties: HashMap<String, Property>, //Hahsmap - Order defined by component
    conditions: HashMap<String, Condition>, //Hahsmap - Order defined by component
    instances: HashMap<String, u16>, //Hahsmap - Only used for fast lookup
}
impl UIS {
    pub fn new() -> Self {
        Self {
            components: IndexMap::new(),
            properties: HashMap::new(),
            conditions: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    pub fn create_component(&mut self, address: &mut String, name: &str, component: Component) {
        address.push('.');
        address.push_str(&name);
        if self.components.contains_key(address){
            if let Some(count) = self.instances.get_mut(address) {
                let instance = *count;
                *count += 1;
                address.push_str(&format!("[{}]", instance));
            }
            else {
                self.instances.insert(address.clone(), 3);
                address.push_str("[2]");
            }
        }
        self.components.insert(address.clone(), component);
        // println!("{}", &address);
    }

    pub fn create_property(&mut self, address: &mut String, property: Property) {
        self.properties.insert(address.to_string(), property);
        match address_split(address) {
            Some((left, right)) => {
                println!(" → Add Property : \"{}\"", right);
                println!(" → Inside Component: \"{}\"", left);
                println!();
            }
            None => println!("No split"),
        }
    }

    pub fn create_condition(&mut self, address: String, condition: Condition) {
        self.conditions.insert(address, condition);
    }

    pub fn get_component(&self, address: &str) -> Option<&Component> {
        self.components.get(address)
    }

    pub fn get_property(&self, address: &str) -> Option<&Property> {
        self.properties.get(address)
    }

    pub fn get_condition(&self, address: &str) -> Option<&Condition> {
        self.conditions.get(address)
    }
}

#[derive(Debug, PartialEq)]
enum PropertyType {
    Integer,
    Decimal,
    ZeroToOne,
    Text,
    Color,
    Switch,
    Structure,
    Option,
    Reference,
}

#[derive(Debug, Default, Clone, Copy)]
struct PropertyFlags(u8);
impl PropertyFlags {
    const ALLOW_ARRAY: u8 = 0b0000_0001;
    const READ_ONLY: u8 = 0b0000_0010;
    const FLAG_C: u8 = 0b0000_0100;
    const FLAG_D: u8 = 0b0000_1000;
    const FLAG_E: u8 = 0b0001_0000;

    fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }
    fn clear(&mut self, flag: u8) {
        self.0 &= !flag;
    }
    fn is_set(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }
}

#[derive(Debug)]
struct Property {
    pub expression: String,
    pub flags: PropertyFlags,
    pub computed: Vec<PropertyType>,
    pub contacts: Vec<String>, //Store short code for the address it will effect (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
}

#[derive(Debug, Default)]
struct Component {
    properties: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    components: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    conditions: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
}

#[derive(Debug)]
pub struct Condition {
    pub expression: String,
}

fn load_uis_file(src: &str) -> io::Result<UISFile> {
    let mut content = String::new();
    BufReader::new(File::open(src)?).read_to_string(&mut content)?;
    Ok(UISFile {
        name: extract_filename(src),
        data: content,
    })
}

fn remove_uis_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_block_comment = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if in_block_comment {
            if let Some(end_idx) = trimmed.find("*/") {
                in_block_comment = false;
            }
        } else if let Some(start_idx) = trimmed.find("/*") {
            result.push_str(&trimmed[..start_idx]);
            in_block_comment = true;
        } else if let Some(start_idx) = trimmed.find("//") {
            result.push_str(&trimmed[..start_idx]);
        } else {
            result.push_str(trimmed);
        }
        result.push('\n');
    }
    result
}

fn flatten_uis(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '{' | '}' => result.push_str(&format!("\n{}\n", c)),
            _ => result.push(c),
        }
    }
    result.lines().filter(|line| !line.trim().is_empty()).map(str::trim).collect::<Vec<_>>().join("\n")
}

fn cleanup_uis(input: &mut String) {
    *input = flatten_uis(&remove_uis_comments(input));
}

fn initialize_uis(name: &str, data: &str, uis: &mut UIS) {
    let mut address = String::new();
    address.push_str(name);
    let mut lines = data.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        let next_trimmed = lines.peek().map(|l| l.trim()).unwrap_or("");

        if trimmed.starts_with('@')
        {
            if trimmed.contains(".[") && trimmed.ends_with("]") 
            {
                // alter condition
            }
        }
        else if trimmed.starts_with('[') || trimmed.contains(':') || next_trimmed.contains('{') {
            if !trimmed.starts_with('[') && !trimmed.contains(':') {
                initialize_component(trimmed, &mut address, uis);
            } else if trimmed.contains(':') && next_trimmed.contains('{') {
                initialize_property(trimmed, &mut address, uis);
            } else if trimmed.contains(':') {
                initialize_property(trimmed, &mut address, uis);
                address_back(&mut address);  // Reset address after property
            } else if trimmed.starts_with('[') {
                initialize_condition(trimmed, &mut address, uis);
            }
        } else if trimmed.starts_with('}') {
            address_back(&mut address); // Move back for closing brace
        }
    }
}

fn initialize_component(trimmed: &str, address: &mut String, uis: &mut UIS) {
    //if new{
    if let Some(name) = trimmed.split_whitespace().last() {
        uis.create_component(address, name, Component::default());
    }
    //}
}

fn initialize_property(trimmed: &str, address: &mut String, uis: &mut UIS) {
    if let Some(first_part) = trimmed.split(':').next() {
        if let Some(last) = first_part.split_whitespace().last() {
            address_forward(address, last, uis);
            uis.create_property(
                &mut address.clone(),
                Property {
                    expression: "1 + 2".to_string(),
                    flags: PropertyFlags::default(),
                    computed: vec![],
                    contacts: vec![],
                },
            );
        }
    }
}

fn initialize_condition(trimmed: &str, address: &mut String, uis: &mut UIS) {
    if let Some(cond) = trimmed.split_whitespace().last() {
        let wrapped = format!("[{}]", cond);
        address_forward(address, &wrapped, uis);
        uis.create_condition(
            address.clone(),
            Condition {
                expression: "true".to_string(), // Placeholder
            },
        );
    }
}

fn address_back(address: &mut String) {
    if let Some(index) = address.rfind('.') {
        address.truncate(index);
    }
}

fn address_forward(address: &mut String, new: &str, uis: &mut UIS) {
    address.push('.');
    address.push_str(new);
    // println!("{}", &address);
}

fn address_split(address: &mut String) -> Option<(&str, &str)> {
    let mut idx = 0;
    for part in address.split('.') {
        if let Some(c) = part.chars().next() {
            if c.is_ascii_lowercase() || c == '\'' {
                if idx == 0 {
                    return None; // no "before" part
                }
                // Split safely using the original string's index
                let before = &address[..idx - 1]; // exclude the dot
                let after = &address[idx..];
                return Some((before, after));
            }
        }
        idx += part.len() + 1; // move past this segment and the dot
    }
    None
}


fn extract_filename(path: &str) -> String {
    // Extract filename from the path
    let filename = path.rsplit_once('/')
        .map_or(path, |(_, after)| after)
        .strip_suffix(".uis")
        .unwrap_or(path)
        .to_owned();

    // Capitalize the first letter of the filename
    let mut filename = filename;
    if let Some(first_char) = filename.chars().next() {
        if first_char.is_lowercase() {
            filename.replace_range(0..1, &first_char.to_uppercase().to_string());
        }
    }
    
    filename
}

////////// NEXT STEP //////////
/// Make sure new addresses don't overwrite old
/// Make addresses "arrayable"
fn main() -> io::Result<()> {
    timer!("All in all it", {
        let mut uis = UIS::new();

        let mut files = vec![
            load_uis_file("Standard.uis")?,
            load_uis_file("tests/test.uis")?,
            // load_uis_file("tests/testTTL.uis")?,
            // load_uis_file("tests/testHTL.uis")?,
        ];

        for file in &mut files {
            timer!(format!("Cleanup {}", file.name), {
                cleanup_uis(&mut file.data);
            });
            timer!(format!("Tokenization {}", file.name), {
                initialize_uis(&file.name, &file.data, &mut uis);
            });
        }

        // println!("{:#?}", uis.components);
        // println!("{:#?}", uis.properties);
        // println!("{:#?}", uis.conditions);
        timer!("get property",{
            uis.get_property("testTTL.mainWindow.button.fill");
        }); 
        // println!("Size of UIS: {} bytes", size_of::<UIS>());
    });
    Ok(())
}