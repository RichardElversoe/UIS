/// In the pursuit of learning rust, I've turned this code into pure spaghetti - Do not use for anything but pure inspiration on how NOT to write the UIS-Engine in rust.

use indexmap::IndexMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::env;

#[macro_export]
macro_rules! timer {
    ($label:expr, $code:block) => {{
        use std::time::Instant;

        let start = Instant::now();
        let result = $code;
        let elapsed = start.elapsed();
        let pr_sec = ((1.0 / elapsed.as_secs_f64().max(f64::MIN_POSITIVE)).round() as u64)
            .to_string()
            .chars()
            .rev()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join(",")
            .chars()
            .rev()
            .collect::<String>();

        // Desired label width
        const LABEL_WIDTH: usize = 24;
        let padded_label = format!("{:<width$}", $label, width = LABEL_WIDTH);

        println!("{} took: {:>8.2?} ({} fps)", padded_label, elapsed, pr_sec);
        result
    }};
}

#[derive(Debug)]
struct UIS {
    components: IndexMap<String, Component>, //Index map - Order is important here!
    properties: IndexMap<String, Property>, //Index map - Order acts seperately from component / condition
    conditions: HashMap<String, Condition>, //Hahsmap - Order defined by component / condition
    instances: HashMap<String, u16>, //Hahsmap - Only used for fast lookup
    
    //For debugging
    debug: bool,
    warnings: Vec<String>,
}
impl UIS {
    pub fn new(main: &str) -> Self {
        let mut uis_self=  Self {
            components: IndexMap::new(),
            properties: IndexMap::new(),
            conditions: HashMap::new(),
            instances: HashMap::new(),
            
            debug: false,
            warnings: vec![],
        };

        match uis_self.load_uis_file(main) {
            Ok(mut content) => {
                uis_self.cleanup_uis(&mut content);
                uis_self.map_uis("".to_string(), "".to_string(), &content);
            }
            Err(err) => {
                uis_self
                    .warnings
                    .push(format!("Failed to load '{}': {}", main, err));
            }
        }

        uis_self
    }
     
    fn cleanup_uis(&self, input: &mut String) {
        *input = self.flatten_uis(&self.remove_uis_comments(input));
    }

    fn load_uis_file(&self, src: &str) -> io::Result<String> {
        let mut content = String::new();

        // First, try relative to current working directory
        if let Ok(file) = File::open(src) {
            BufReader::new(file).read_to_string(&mut content)?;
            return Ok(content);
        }

        // Fallback: try relative to executable directory
        if let Ok(exe_dir) = env::current_exe().and_then(|p| Ok(p.parent().unwrap().to_path_buf())) {
            let full_path = exe_dir.join(src);
            if let Ok(file) = File::open(&full_path) {
                BufReader::new(file).read_to_string(&mut content)?;
                return Ok(content);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not find file '{}'", src),
        ))
    }

    fn remove_uis_comments(&self, input: &str) -> String {
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

    fn flatten_uis(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for c in input.chars() {
            match c {
                ';' => result.push('\n'),
                '{' | '}' => {
                    result.push('\n');
                    result.push(c);
                    result.push('\n');
                }
                _ => result.push(c),
            }
        }
        result
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn insert_uis_file(&mut self, line: &str, origin: &mut String, address: &mut String){
        if let Some(mut source) = line.split_whitespace().next(){
            source = source.trim_matches('"');
            match self.load_uis_file(source) {
                Ok(mut content) => {
                    self.cleanup_uis(&mut content);
                    if line.split_whitespace().last() != line.split_whitespace().next() {

                        //LØSNING? 
                            //OPDATER ADRESSE 'NULPUNKT' VED NAVNGIVET INDSÆTNING
                        // address_forward(address, line.split_whitespace().last().expect(source));
                        self.add_component(address, Component::new(&origin,"~"), line.split_whitespace().last().expect("Expected the insert material to have a name"));
                        self.map_uis(line.split_whitespace().last().expect(source).to_string(), address.to_string(), &content);
                    }
                    else {
                        self.map_uis("".to_string(), address.to_string(), &content);
                        address_forward(address, "INSERTED");
                    }
                }
                Err(err) => {
                    self
                        .warnings
                        .push(format!("Failed to load/insert '{}': {}", source, err));
                }
            }
        }
    }

    fn map_uis(&mut self, mut origin: String, mut address: String, data: &str) {
        let mut lines = data.lines().peekable();
        
        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            let next_trimmed = lines.peek().map(|l| l.trim()).unwrap_or("");
            if trimmed.starts_with("{") {
                continue;
            } else if trimmed.starts_with('}') {
                address_back(&mut address); // Move back for closing brace
            } else if trimmed.starts_with('"') && next_trimmed.contains('{') {
                if origin != ""
                {
                    address_forward(&mut address, &origin);
                }
                self.insert_uis_file(trimmed, &mut origin , &mut address);
            } else if trimmed.starts_with('@') {
                if trimmed.contains(".[") && trimmed.ends_with("]") {
                    // alter condition
                } else {
                    // alter component
                }
            } else if trimmed.starts_with('[') || trimmed.contains(':') || next_trimmed.contains('{') {
                if !trimmed.starts_with('[') && !trimmed.contains(':') {
                    self.map_component(trimmed, &mut origin, &mut address);
                } else if trimmed.contains(':') {
                    self.map_property(trimmed, &mut address);
                    if !next_trimmed.contains('{'){
                        address_back(&mut address);
                    }
                } else if trimmed.starts_with('[') {
                    self.map_condition(trimmed, &mut address);
                }
            } else {
                self.warnings.push(format!("unexpected line: {}", line));
            }
        }
    }
    
    fn map_component(&mut self, trimmed: &str, origin: &mut String, address: &mut String) {
        if let Some(a) = trimmed.split_whitespace().next(){
            if let Some(name) = trimmed.split_whitespace().last() {
                self.add_component(address, Component::new(&origin, a), name);
            }
        }
    }

    fn map_property(&mut self, trimmed: &str, address: &mut String) {
        if let Some((first_part, last_part)) = trimmed.split_once(':') {
            if let Some(last) = first_part.split_whitespace().last() {
                address_forward(address, last);

                if trimmed.starts_with('+') {
                    self.add_property(
                        &address.clone(),
                        last_part.trim_start(),
                    );
                }
                else {
                    self.set_property(
                        &address.clone(),
                        last_part.trim_start(),
                    );
                }
            }
        }
    }

    fn map_condition(&mut self, trimmed: &str, address: &mut String) {
        if let Some(cond) = trimmed.split_whitespace().last() {
            let wrapped = format!("[{}]", cond);
            address_forward(address, &wrapped);
            self.add_condition(
                &mut address.clone(),
                Condition::default(),
            );
        }
    }

    pub fn run<F>(&self, mut logic: F) where F: FnMut(), {
        loop {
            //Starting logic

            //Main logic
            logic();

            //Ending logic
            self.debug();
        }

        
    }

    fn debug(&self) {
        if self.debug == true && self.warnings.len() > 0{
            println!("Warnings:");
            for warn in &self.warnings {
                println!("{}", warn);
            }
        }
    }

    pub fn debug_list(&self){
        for C in &self.components {
            println!("{}", C.0);
            let mut checked: Vec<String> = vec![];
            self.debug_print_properties(C.1, &mut checked);
        }        
    }

    pub fn debug_print_properties(&self, comp: &Component , skip: &mut Vec<String>) {
        for prop in comp.properties.iter(){
            if !skip.contains(prop){
                match self.get_property(&format!("{}.{}", comp.address, prop)) {
                    None => {
                        print!("TNF!");
                    }
                    _ => {
                        print!("\t{:#?}", self.get_property(&format!("{}.{}", comp.address, prop)).unwrap().of_type);
                    }
                }
                println!("\t{}", prop);
                skip.push(prop.clone());
            }
        }
        
        if !comp.ancestor.contains('~') {
            println!("\t------------ ↓ inherited from {} ↓ -----------", comp.ancestor);
            if let Some(ancestor) = self.get_ancestor(comp) {
                self.debug_print_properties(ancestor, skip);
            } else {
                println!("\tAncestor not found: {}", comp.ancestor);
            }
        }
    }

    //COMPONENTS
    pub fn component<'a>(&'a mut self, address: &'a str) -> ComponentHandle<'a> {
        ComponentHandle { address, uis: self }
    }
    
    pub fn add_component(&mut self, address: &mut String, mut component: Component, name: &str) {
        let mut is_root = true;

        if address != "" {
            is_root = false;
            address.push('.');
        }

        //Advance address
        address.push_str(&name);

        //Check for multiple instances
        if self.components.contains_key(address){
            if let Some(count) = self.instances.get_mut(address) {
                let instance = *count;
                *count += 1;
                address.push_str(&format!("[{}]", instance));
            }
            else {
                self.instances.insert(address.to_string(), 3);
                address.push_str("[2]");
            }
        }
        component.address = address.to_string();
        self.components.insert(address.to_string(), component);

        //Inform its parent
        if !is_root{
            if let Some((container_key, name)) = address_split(address) {
                if self.components.contains_key(container_key) {
                    self.components.get_mut(container_key).unwrap().components.push(name.to_string());
                } else if self.conditions.contains_key(container_key) {
                    self.conditions.get_mut(container_key).unwrap().components.push(name.to_string());
                }
                else {
                    println!("Error inserting component: {} \t\t into: {}", address, container_key);
                }
            } else {
                println!("Error inserting component: {}", address);
            }
        }
    }
    
    pub fn get_component(&self, address: &str) -> Option<&Component> {
        self.components.get(address)
    }

    pub fn get_ancestor(&self, from: &Component) -> Option<&Component>{
        self.components.get(&from.ancestor)
    }

    //PROPERTIES
    pub fn add_property(&mut self, address: &str, expression: &str) {
        self.add_property_internal(address, Property::new(expression));
    }

    pub fn add_property_of_type(&mut self, address: &str, expression: &str, of_type: PropertyType) {
        self.add_property_internal(address, Property::new_of_type(of_type, expression));
    }

    fn add_property_internal(&mut self, address: &str, property: Property) {
        self.properties.insert(address.to_string(), property);

        if let Some((container_key, name)) = address_split(address) {
            if let Some(container) = self.components.get_mut(container_key) {
                container.properties.push(name.to_string());
            } else if let Some(condition) = self.conditions.get_mut(container_key) {
                condition.properties.push(name.to_string());
            } else {
                println!("Error inserting property: Could not find parent for {}", address);
            }
        } else {
            println!("Error inserting property: {}", address);
        }
    }

    pub fn set_property(&mut self, address: &str, expression: &str) {
        if self.properties.get_mut(address).is_some(){            
            if is_static_expression(expression){
                self.properties.get_mut(address).unwrap().expression.clear();
            }
            self.properties.get_mut(address).unwrap().expression.push(expression.to_string());
        } else if let Some(split)= address_split(address) {
            if let Some(component) = self.get_component(split.0){
                if let Some(ancestor) = self.get_ancestor_with_property(component, split.1){
                    if let Some(ancestor_property) = self.get_property(&format!("{}.{}", ancestor.address, split.1)) {
                        let mut new_property_instance = Property::new_of_type(ancestor_property.of_type, expression);
                        self.add_property_internal(address, new_property_instance);
                    }
                }
                else {
                    self.warnings.push(format!("Omitted: {}\t\tReason: Coud not find {}", expression, address));
                }
            }
            //self.warnings.push(format!("{} was not found directly in {}", split.1, split.0));
        }
    }

    // pub fn get_property(&self, address: &str) -> Option<&Property> {
    //     if self.properties.contains_key(address){
    //         self.properties.get(address)
    //     }
    //     else {
    //         while  {
    //             //
    //             sd
    //         }
    //     }
    // }

    pub fn get_property(&self, address: &str) -> Option<&Property> {
        self.properties.get(address)
    }

    fn get_ancestor_with_property(&self, caller_address: &Component, property_name: &str) -> Option<&Component> {
        if caller_address.ancestor != "~" {
            if let Some(ancestor) = self.get_ancestor(caller_address) {
                if self.properties.contains_key(&format!("{}.{}", ancestor.address, property_name)){
                    Some(ancestor)
                }
                else {
                    self.get_ancestor_with_property(ancestor, property_name)
                }
            }
            else {
                None
            }
        }
        else {
            None
        }
    }

    pub fn get_property_addresses(&self, from: &Component) -> Vec<String>{
        let mut a: Vec<String> = vec![];
        return a;
    }

    fn compute_property(){

    }
    
    //CONDITIONS
    pub fn add_condition(&mut self, address: &str, condition: Condition) {
        self.conditions.insert(address.to_string(), condition);
    
        if let Some((container_key, name)) = address_split(address) {
            if self.components.contains_key(container_key) {
                self.components.get_mut(container_key).unwrap().conditions.push(name.to_string());
            } else if self.conditions.contains_key(container_key) {
                self.conditions.get_mut(container_key).unwrap().conditions.push(name.to_string());
            } else {
                println!("Error inserting condition: Could not find parent for {}", address);
            }
        } else {
            println!("Error inserting condition: {}", address);
        }
    }

    pub fn get_condition(&self, address: &str) -> Option<&Condition> {
        self.conditions.get(address)
    }

}

#[derive(Debug, PartialEq, Clone, Copy)]
enum PropertyType {
    // ===== NUMBER ===== //
    Number,                 //should be variable in size down the line
    NumberUnsigned,         //should be variable in size down the line
    NumberUnsigned8,
    NumberUnsigned16,
    NumberUnsigned32,
    NumberUnsigned64,
    NumberUnsigned128,
    NumberSigned,           //should be variable in size down the line
    NumberSigned8,
    NumberSigned16,
    NumberSigned32,
    NumberSigned64,
    NumberSigned128,
    NumberFloatingpoint,
    NumberFloatingpoint32,
    NumberFloatingpoint64,
    NumberZeroToOne,        //Acts like a floatingpoint without the exponent
    NumberBit,
    // ===== TEXT ===== //
    Text,
    // ===== COLOR ===== //
    Color,                  //should be variable in sizes down the line
    ColorRGB8,
    ColorRGB16,
    ColorRGB32,
    ColorRGBA8,
    ColorRGBA16,
    ColorRGBA32,
    // ===== OPTION ===== //
    Option,
    // ===== REFERENCE ===== //
    Reference,
    // ===== Expression ===== //
    Expression,             //Stores a raw expression, that will be inserted directly into the expression of whatever property refers to it.
    // ===== STRUCTURE ===== //
    Structure,              //Store a short code reference to sub properties, similar to how components does it.
}

/* Remove?

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

*/

#[derive(Debug)]
struct Property {
    expression: Vec<String>,
    of_type: PropertyType,
    computed: Vec<PropertyType>,
    contacts: Vec<String>, //Store short code for the address that are relying on it
}
impl Property {
    pub fn new(expression: &str) -> Self {
        Self {
            expression: vec![expression.to_string()],
            of_type: Self::compute_type(expression),
            computed: Vec::new(),
            contacts: Vec::new(),
        }
    }

    pub fn new_of_type(property_type: PropertyType, expression: &str) -> Self {
        Self {
            of_type: property_type,
            expression: vec![expression.to_string()],
            computed: Vec::new(),
            contacts: Vec::new(),
        }
    }

    fn compute_type(expression: &str) -> PropertyType {
        if expression.contains('"') {
            PropertyType::Text
        } else if expression.contains('#') {
            PropertyType::Color
        } else if expression.contains("'") && is_static_expression(expression) {
            PropertyType::Option
        } else {
            PropertyType::Number // Default
        }
    }
    
}

#[derive(Debug, Default)]
struct Component {
    address: String,
    ancestor: String,
    properties: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    components: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    conditions: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
}
impl Component {
    pub fn new(origin: &str, ancestor: &str) -> Self {
        let mut component = Self {
            address: String::new(),
            ancestor: String::new(),
            properties: Vec::new(), //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
            components: Vec::new(), //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
            conditions: Vec::new(), //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")   
        };
        
        if origin != ""
        {
            component.ancestor = format!("{}.{}", origin.to_string(), ancestor.to_string());
        }
        else {
            component.ancestor = ancestor.to_string();
        }
        
        component
    }
}

#[derive(Debug, Default)]
pub struct Condition {
    pub expression: String,
    pub is_true: bool,

    properties: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    components: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
    conditions: Vec<String>, //Store short code for the address (e.g. Component address = "cA.cAA", Porperty address = "cA.cAA.pA", would simply store at ".pa")
}

pub struct ComponentHandle<'a> {
    pub address: &'a str,
    uis: &'a mut UIS,
}
impl<'a> ComponentHandle<'a> {
    pub fn property<'b>(&'b mut self, name: &str) -> PropertyHandle<'b> {
        let full_key = format!("{}.{}", self.address, name);
        PropertyHandle {
            address: full_key,
            uis: self.uis,
        }
    }
}

pub struct PropertyHandle<'a> {
    pub address: String,
    uis: &'a mut UIS,
}
impl<'a> PropertyHandle<'a> {
    pub fn set<T: ToString>(&mut self, value: T) {
        if let Some(prop) = self.uis.properties.get_mut(&self.address) {
            prop.expression.push(value.to_string());
        } else {
            // Maybe auto-insert, or warn:
            println!("Property '{}' not found", self.address);
        }
    }
}

/// Returns `true` if there's any non-whitespace content outside quotes.
fn is_static_expression(expr: &str) -> bool {
    fn tokenize(s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut parens = 0;
        for c in s.chars() {
            match c {
                '(' => {
                    parens += 1;
                    current.push(c);
                }
                ')' => {
                    parens -= 1;
                    current.push(c);
                }
                '|' | '&' if parens == 0 => {
                    if !current.trim().is_empty() {
                        tokens.push(current.trim().to_string());
                    }
                    tokens.push(c.to_string());
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        if !current.trim().is_empty() {
            tokens.push(current.trim().to_string());
        }
        tokens
    }

    fn is_token_static(token: &str) -> bool {
        let token = token.trim();
        if token.starts_with('\'') || token.starts_with('"') {
            return true;
        }

        // Numeric constant
        if token.parse::<f64>().is_ok() {
            return true;
        }

        // Parentheses — evaluate recursively
        if token.starts_with('(') && token.ends_with(')') {
            return is_static_expression(&token[1..token.len()-1]);
        }

        // Operators like +, -, *, /
        if let Some(op_pos) = token.find(|c| "+-*/".contains(c)) {
            let (left, right) = token.split_at(op_pos);
            return is_token_static(left) && is_token_static(&right[1..]);
        }

        // If it’s just a word, it's a dependency
        false
    }

    let tokens = tokenize(expr);

    // Evaluate tokens based on | and &
    let mut result = None;
    let mut op = None;

    for token in tokens {
        match token.as_str() {
            "|" | "&" => op = Some(token),
            _ => {
                let token_result = is_token_static(&token);
                result = Some(match (result, op.as_deref(), token_result) {
                    (None, _, b) => b,
                    (Some(a), Some("|"), b) => a || b,
                    (Some(a), Some("&"), b) => a && b,
                    _ => token_result,
                });
            }
        }
    }

    result.unwrap_or(false)
}


fn address_back(address: &mut String) {
    if let Some(index) = address.rfind('.') {
        address.truncate(index);
    }
    else {
        address.clear();
    }
}

fn address_forward(address: &mut String, new: &str) {
    address.push('.');
    address.push_str(new);
}

fn address_split(address: &str) -> Option<(&str, &str)> {
    let bytes = address.as_bytes();
    let mut breakup = 0;
    let mut address_index = 0;

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' {
            if let Some(&c) = bytes.get(address_index) {
                if (b'A'..=b'Z').contains(&c) || c == b'[' {
                    breakup = i;
                } else {
                    break;
                }
            }
            address_index = i + 1;
        }
    }
    
    if breakup > 0 {
        Some((&address[..breakup], &address[breakup + 1..]))
    } else {
        None
    }
}

//NEXT STEPS:
    //✅ Cleanup initialization -->    UIS::new("tests/cool.uis"); //handles the loading
    //✅ Add inclusion/use functionality to file
    //☑️ Add Ancestor reference to component properties (if they cant be found, they might be part of the ancestor)
    //☑️ Add auto renew on dependent ancestrial properties

    //Refactor... soon... likely now
        //The inclusion/use need serious rework / rethinking
        //Mapping needs rework
        //Most things need a bit of cleanup
        //find common language for components, properties and conditions
fn main() -> io::Result<()> {
    
    timer!("All in all", {
        let mut uis = UIS::new("tests/test.uis");
        uis.debug_list();
        uis.debug = true;
        uis.debug();

        /*
        // let mut files = vec![
        //     // load_uis_file("tests/basics.uis")?,
        //     load_uis_file("Standard.uis")?,
        //     // load_uis_file("tests/test.uis")?,
        //     // load_uis_file("tests/testTTL.uis")?,
        //     // load_uis_file("tests/testHTL.uis")?,  
        // ];

        // for file in &mut files {
        //     timer!(format!("Cleanup {}", file.name), {
        //         cleanup_uis(&mut file.data);
        //     });
        //     timer!(format!("Map {}", file.name), {
        //         map_uis(&file.name, &file.data, &mut uis);
        //     });
        // }
        
        
        // timer!(format!("Go through all components and their first property"), {
        //     for comp in uis.components {
        //         let x = uis.get_property(&comp.1.properties[0]);
        //         comp uis::Component("")
        //     }
        // });

        // let mut uis_component = uis.component("Test.MainWindow");
        // uis_component.property("width").set(800);
        // uis_component.property("height").set("600");

        // timer!("Set Button Label A ", {
        //     uis.component("Test.Button").property("label").set("Hello World");
        // });

        // timer!("Set Button Label B ", {
        //     uis.set_property("Test.Button.Text.write", "Hello World");
        // });

        // timer!("Get Button Label ", {
        //     uis.get_property("Test.Button.label");
        // });


        // timer!("Go through and alter all properties", {
        //     for prop in uis.properties.iter_mut() {
        //         prop.1.expression = String::from("hi");
        //     }
        // });
        // println!("Size of UIS: {} bytes", uis.);
        // println!("Size of UIS: {} bytes", size_of::<uis>());
        
        
        
        // println!("{:#?}", uis.components);
        // println!("{:#?}", uis.properties);
        // println!("{:#?}", uis.conditions);
        // println!("{:#?}", uis);
        // let x;
        // timer!("Get testValue", {
        //     x = &uis.get_property("Button.testValue").unwrap().expression;
        // });
        // println!("\n{} \n{:?}\n", "Button.testValue", x);

        */
        
    });

    // std::thread::sleep(std::time::Duration::from_secs(30));
    Ok(())
}