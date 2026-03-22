use indexmap::{IndexMap};
use std::collections::HashMap;
use std::f32::consts::E;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path};
use std::{env};
 
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

///////////////////////////////////////////////////////////////////////////////////////////
///                                         UIS                                         ///
///////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Uis {
    addressor: Address,

    components: Vec<Component>,
    component_map: HashMap<String, ID>,
    properties: Vec<Property>,
    property_map: HashMap<String, ID>,

} impl Uis {
    pub fn new() -> Self{
        Self {
            addressor: Address::new(),
            components: Vec::new(),
            component_map: HashMap::new(),
            properties: Vec::new(),
            property_map: HashMap::new(),
        }
    }
    pub fn new_from_string(data: String) -> Self {
        let mut uis = Uis::new();
        uis.tokenize_data(data);
        uis
    }
    pub fn new_from_file(source: &Path) -> io::Result<Self> {
        let mut content = String::new();

        // First, try relative to current working directory
        if let Ok(file) = File::open(source) {
            BufReader::new(file).read_to_string(&mut content)?;
            let uis = Uis::new_from_string(content);
            return Ok(uis);
        }

        // Fallback: try relative to executable directory
        if let Ok(exe_dir) = env::current_exe().and_then(|p| Ok(p.parent().unwrap().to_path_buf())) {
            let full_path = exe_dir.join(source);
            if let Ok(file) = File::open(&full_path) {
                BufReader::new(file).read_to_string(&mut content)?;
                content = content.replace("\r\n", "\n").replace("\r", "\n");
                let uis = Uis::new_from_string(content);
                // uis.path = exe_dir.into_os_string().into_string().unwrap();
                return Ok(uis);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not find file '{:?}'", source),
        ))
        
    }
    
    fn insert_uis(){

    }
    fn insert_component(&mut self, c: Component){
        self.component_map.insert(self.addressor.current.clone(), c.id);
        self.components.push(c);
    }
    fn insert_property(&mut self, p: Property){
        let address = self.addressor.current.clone();
        
        match p.origin {
            Origin::Component(o) => {
                if let Some(c) = self.components.get_mut(o) {
                    c.properties.insert(address.clone(), p.id.clone());
                    self.property_map.insert(address, p.id.clone());
                    self.properties.push(p);
                } else {
                    todo!("return some error - should never happen");
                }
            },
            Origin::Root => {
                self.property_map.insert(address, p.id.clone());
                self.properties.push(p);
            },
        }
        
    }

    fn tokenize_data(&mut self, data: String){
        let mut is_new_line: bool = false;

        let mut continue_at_next_line = false;

        let mut indent_current: usize = 0;
        let mut indent_previous: usize = 0;
        let mut curl_depth: usize = 0;
        let mut cond_depth: usize = 0;
        let mut paren_depth: usize = 0;
        let mut token: String = String::new();
        let mut token_type: TokenTypes = TokenTypes::Base;

        let unified_data = data.replace("\r\n", "\n").replace("\r", " ");
        let mut iter = unified_data.chars().peekable();

        while let Some(ch) = iter.next() {
            let next_char = iter.peek();

            match ch {
                '"' => construct_simple_quote(&mut iter, &mut token),
                '\'' => construct_macro(&mut iter, &mut token),
                '/' if next_char == Some(&'*') => skip_multiline_comment(&mut iter),
                '/' if next_char == Some(&'/') => skip_singleline_comment(&mut iter),

                '{' | '}' | '[' | ']' | '(' | ')' => set_depth(ch, &mut token, &mut curl_depth,&mut cond_depth, &mut paren_depth),

                ',' | '+' | '-' | '*' | '/' | '|' | '&' => {
                    continue_at_next_line = true;
                    token.push(ch);
                }

                ':' => {
                    //token.push(ch);
                    self.evaluate_token_base(&mut token);
                    token_type = TokenTypes::Expression;
                }
                
                '\t' if is_new_line => indent_current += 1,
                '\n' | '\r' => {
                    if !token.is_empty() && !continue_at_next_line && curl_depth == 0 && cond_depth == 0 && paren_depth == 0 {
                        match token_type {
                            TokenTypes::Base        => self.evaluate_token_base(&mut token),
                            TokenTypes::Expression  => self.evaluate_token_expression(&mut token),
                        }
                    } else if !token.is_empty() {
                        continue; //ignore empty lines
                    }

                    token_type = TokenTypes::Base;
                    continue_at_next_line = false;
                    is_new_line = true;
                    indent_previous = indent_current;
                    indent_current = 0;
                }

                _ => {
                    token.push(ch);
                    if ch != ' ' || ch != '\t' {
                        continue_at_next_line = false;
                    }
                    if is_new_line {
                        let steps = indent_previous.saturating_sub(indent_current);
                        for _ in 0..steps {
                            self.addressor.back();
                        }
                        is_new_line = false;
                    }
                },
            }
        }
        match token_type {
            TokenTypes::Base        => self.evaluate_token_base(&mut token),
            TokenTypes::Expression  => self.evaluate_token_expression(&mut token),
        }
        #[inline(always)]
        fn skip_multiline_comment<I: Iterator<Item = char>>(iter: &mut std::iter::Peekable<I>) {
            iter.next(); // consume '*'
            while let Some(n) = iter.next() {
                if n == '*' && iter.peek() == Some(&'/') {
                    iter.next(); // consume '/'
                    break;
                }
            }
        }
        #[inline(always)]
        fn skip_singleline_comment<I: Iterator<Item = char>>(iter: &mut std::iter::Peekable<I>) {
            iter.next(); // consume second '/'
            while let Some(n) = iter.next() {
                if n == '\n' {
                    break;
                }
            }
        }
        fn construct_simple_quote<I: Iterator<Item = char>>(iter: &mut std::iter::Peekable<I>, token: &mut String) {
            token.push('"'); //add first quotationmark
            while let Some(n) = iter.next() {
                token.push(n);
                if n == '"' {
                    break;
                }
            }
        }
        fn construct_macro<I: Iterator<Item = char>>(iter: &mut std::iter::Peekable<I>, token: &mut String) {
            token.push('\''); //add first optionmark
            while let Some(n) = iter.next() {
                token.push(n);
                if n == '\'' {
                    break;
                }
            }
        }

        #[inline(always)]
        fn set_depth(ch: char, token: &mut String, curl_depth: &mut usize, cond_depth: &mut usize, paren_depth: &mut usize) {
            token.push(ch);
            match ch {
                '{' => *curl_depth += 1,
                '}' => *curl_depth = curl_depth.saturating_sub(1),
                '[' => *cond_depth += 1,
                ']' => *cond_depth = cond_depth.saturating_sub(1),
                '(' => *paren_depth += 1,
                ')' => *paren_depth = paren_depth.saturating_sub(1),
                _ => println!("unexpected char when inserting depth"),
            }
        }    
    }
    fn evaluate_token_expression(&mut self, token: &mut String) {
        // match address.is_type {
        //     BaseTypes::Component => todo!("should never be the case"),
        //     BaseTypes::Property => print!("{}", address.current),
        //     BaseTypes::Macro => print!("{}", address.current),
        //     BaseTypes::State => print!("{}", address.current),
        //     BaseTypes::Root => print!("ROOT"),
        // }
        // println!("\t{token}");
        token.clear();        
    }
    fn evaluate_token_base(&mut self, token: &mut String) {

        //Needs to be reworked
        
        /*
        if token.trim().is_empty() {
            return; // ignore blank tokens
        }

        let mut parts = token.split_whitespace();
        let first = parts.next().unwrap();
        let second = parts.next();

        let derivative: Option<String>;
        let name: &str;

        match (first, second) {
            // Single token
            (tok, None) if tok.starts_with('@') => {
                self.address.reference = Some(tok.trim_start_matches('@').to_string());
                derivative = Some(tok.to_string());
                name = tok;
            }
            (tok, None) if starts_with_lowercase(tok) => {
                self.address.reference = None;
                derivative = Some(tok.to_string());
                name = tok;
            }
            (tok, None) => {
                self.address.reference = None;
                derivative = None;
                name = tok;
            }

            // Two or more tokens
            (tok1, Some(tok2)) if tok1 == "+" => {
                self.address.reference = None;
                derivative = None;
                name = tok2;
            }
            (tok1, Some(tok2)) => {
                self.address.reference = None;
                derivative = Some(tok1.to_string());
                name = tok2;
            }
        }

        match Address::evaluate_type(name) {
            BaseTypes::Component => Component::new(self, name, self.components.get_mut(derivative).cloned()),
            _ => {
                println!("--- {:?} --- {:?} --- {:?}", self.address.current, name, derivative)
                // if address.exists(self) {
                //     address.forward(name);
                // }
            },
        }

        token.clear();
        */
    }  


    fn evaluate_origin_address(&mut self, address: &str) -> Option<String> {
        //Quick GPT implimentation -> needs a propper implimentation soon, but it works for now - Richard E. 25 jan. 2026

        let bytes = address.as_bytes();

        // 1. Find last '.'
        let mut end = None;
        for i in (0..bytes.len()).rev() {
            if bytes[i] == b'.' {
                end = Some(i);
                break;
            }
        }

        let end = match end {
            Some(i) => i,
            None => return None,
        };

        // 2. Walk forward, stopping at first lowercase segment
        let mut last_valid_end = 0;

        let mut i = 0;
        while i < end {
            let b = bytes[i];

            if b == b'.' {
                last_valid_end = i;
            } else if b.is_ascii_lowercase() {
                break;
            }

            i += 1;
        }

        if last_valid_end == 0 {
            None
        } else {
            Some(address[..last_valid_end].to_string())
        }
    }

    fn evaluate_origin(&mut self, address: &str) -> Origin {
        match self.evaluate_origin_address(address) {            
            Some(origin_address) => {
                match self.component_map.get(&origin_address) {
                    Some(origin_id) => return Origin::Component(*origin_id),
                    None => return Origin::Root,
                }
            },
            None => return Origin::Root
        }
    }
}

#[derive(Debug, PartialEq)]
enum TokenTypes {
    Base,
    Expression,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseTypes {
    Root,
    Component,
    Property,
    Macro,
}

#[derive(Debug, Clone)]
pub struct Address {
    current: String,
    is_type: BaseTypes,
} impl Address { 
    pub fn new() -> Self{
        Self { 
            current: String::new(),
            is_type: BaseTypes::Root,
        }
    }
    pub fn new_from_str(at: &str) -> Self{
        let mut a = Address::new();
        a.current = at.to_string();
        return a;
    }
    pub fn root() -> Self {
        Address::new()
    }
    
    pub fn forward(&mut self, next: &str) {
        if !self.current.is_empty() {
            self.current.push('.');
        }
        if next.contains('.') {
            self.current.push('(');
            self.current.push_str(next);
            self.current.push(')');
        } else {
            self.current.push_str(next);
        }

        self.evaluate_own_type();
    }
    pub fn back(&mut self) {
        let cur: &str = &self.current;
        match self.current.rfind('.') {
            Some(pos) => {
                self.current = cur[..pos].to_string();
                self.evaluate_own_type();
            },
            None => {
                self.current = "".to_string();
                self.is_type = BaseTypes::Root;
            }
        }
    }
    /*
    pub fn evaluate_current_origin(&self, uis: &Uis) -> Origin{
        let bytes = self.current.as_bytes();
        let mut i = bytes.len();
        let mut end = i + 1;

        // Walk backwards looking for ".X" where X is uppercase
        while i >= 2 {
            if bytes[i - 1] == b'.' {
                // character right after the dot
                let next = bytes[i];
                if next >= b'A' && next <= b'Z' {
                    // now find the end of that segment
                    
                    while end < bytes.len() && bytes[end] != b'.' {
                        end += 1;
                    };
                    
                }
            }
            i -= 1;
        }
        // First segment case (no dot before it)
        if !bytes.is_empty() {
            let c = bytes[0];
            if c >= b'A' && c <= b'Z' {
                return Origin::Component(*uis.component_map.get(&self.current[..end]).unwrap());
            }
        }
        
        return Origin::Root
    }
    */
    pub fn evaluate_type(from: &str) -> BaseTypes{
        let last = match from.rsplit('.').next() {
            Some(part) => part,
            None => return BaseTypes::Root,
        };

        let last_type = match last.chars().next() {
            Some(c) if c.is_uppercase() => BaseTypes::Component,
            Some(c) if c == '\'' => BaseTypes::Macro,
            Some(c) if c.is_lowercase() => BaseTypes::Property,
            _ => BaseTypes::Root, // fallback (numbers, symbols, etc.)
        };
        last_type
    }
    fn evaluate_own_type(&mut self) {
        self.is_type = Address::evaluate_type(&self.current);
    }
    pub fn get_name(&self) -> String {
        match self.current.rsplit('.').next() {
            Some(name) => return name.to_string(),
            None => return "Root".to_string(),
        }
    }
    /*pub fn get_parent(&self, uis: &Uis) -> Origin{
        let current: &str = &self.current;
        for segments in current.split('.') {
            
        }
    }*/
    pub fn exists(&mut self, uis: &Uis) -> bool{
        let address = &self.current;
        match self.is_type {
            BaseTypes::Root => true,
            BaseTypes::Component => uis.component_map.get_key_value(address).is_some(),
            BaseTypes::Property => uis.property_map.get_key_value(address).is_some(),
            BaseTypes::Macro => todo!(),
        }
    }

}

type ID = usize;
#[derive(Debug, Clone, Copy)]
enum Origin {
    Component(ID),
    Root,
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                      Component                                      ///
///////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
struct Component {
    pub id: ID, //Component ID
    pub address: String,
    pub ancestor_id: Option<ID>,

    pub components: HashMap<String, ID>,
    pub properties: HashMap<String, ID>,

} impl Component {
    pub fn new(uis: &mut Uis, name: &str, ancestor_id: Option<ID>){
        Component::enter_new(uis, name, ancestor_id);
        uis.addressor.back();
    }

    pub fn enter_new(uis: &mut Uis, name: &str, ancestor_id: Option<ID>) {
        uis.addressor.forward(name);
        let address = uis.addressor.current.clone();
        let id = uis.components.len();
        
        let c = Self {
            id,
            address: address.clone(),
            ancestor_id: ancestor_id,
            components: HashMap::new(),
            properties: HashMap::new(),
        };
        
        Component::add_default_propperties(uis, &c);
        uis.insert_component(c);
    }

    fn add_default_propperties(uis: &mut Uis, to: &Component) {
        Property::new(uis, "self", PropertyValue::Reference(to.id));
        // Property::new(uis, "parent", PropertyValue::Reference(uis.addressor.));
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                       PROPERTY                                      ///
///////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Property {
    id: ID,
    origin: Origin,
    value: PropertyValue,
    expression: Option<Vec<String>>,
    macros: Option<IndexMap<String, Option<String>>>,

} impl Property {
    pub fn new(uis: &mut Uis, name: &str, value: PropertyValue){
        Property::enter_new(uis, name, value);
        uis.addressor.back();
    }
    pub fn enter_new(uis: &mut Uis, name: &str, value: PropertyValue){
        uis.addressor.forward(name);
        let address = uis.addressor.current.clone();
        
        let id = uis.properties.len();
        let p = Self {
            id,
            origin: uis.evaluate_origin(&address),
            value,
            expression: None,
            macros: None,
        };

        uis.insert_property(p);
    }
    pub fn new_from_address(uis: &mut Uis, address: String, name: &str, value: PropertyValue){
        Property::enter_new_from_address(uis, address, name, value);
        uis.addressor.back();
    }
    pub fn enter_new_from_address(uis: &mut Uis, address: String, name: &str, value: PropertyValue){
        uis.addressor.current = address;
        Property::enter_new(uis, name, value);
    }

    /*
    fn new_at_origin(uis: &mut Uis, name: &str, value: PropertyValue, origin: &mut Origin){
        let p: Property;
        let address: Address;
        let id = uis.properties.len();
        match origin {
            Origin::Component(x) => {
                p = Self {
                    id,
                    origin: origin.clone(),
                    value,
                    expression: None,
                    macros: None,
                };
            },
            Origin::Root => {
                p = Self {
                    id,
                    origin: origin.clone(),
                    value,
                    expression: None,
                    macros: None,
                };

                uis.property_map.insert(address.current.clone(), id);
                uis.properties.push(p);
            }
        }
    } 
    */

    fn set_expression(expression: String) {

    }
    fn set_value(value: PropertyValue) {

    }
}
#[derive(Debug)]
pub enum PropertyValue {
    NumberDynamicU8(Vec<u8>),
    NumberDynamicU16(Vec<u16>),
    NumberDynamicU32(Vec<u32>),
    NumberDynamicU64(Vec<u64>),
    NumberDynamicI8(Vec<i8>),
    NumberDynamicI16(Vec<i16>),
    NumberDynamicI32(Vec<i32>),
    NumberDynamicI64(Vec<i64>),
    NumberDynamicI128(Vec<i128>),
    NumberDynamicF32(Vec<f32>),
    Text(Vec<String>),
    Option(Vec<usize>),
    Color(Vec<(u8,u8,u8,u8)>),
    References(Vec<ID>),
    Reference(ID),
    Expression(usize),
    Structure(Vec<ID>),
    Undefined,
} impl PropertyValue {
    /// Insert an integer value, promoting vector type if needed
    fn push_int(&mut self, value: i128) {
        match self {
            PropertyValue::NumberDynamicU8(vec) => {
                if value <= u8::MAX as i128 {
                    vec.push(value as u8);
                } else {
                    let mut new_vec: Vec<u16> = vec.iter().map(|&x| x as u16).collect();
                    new_vec.push(value as u16);
                    *self = PropertyValue::NumberDynamicU16(new_vec);
                }
            }
            PropertyValue::NumberDynamicU16(vec) => {
                if value <= u16::MAX as i128 {
                    vec.push(value as u16);
                } else {
                    let mut new_vec: Vec<u32> = vec.iter().map(|&x| x as u32).collect();
                    new_vec.push(value as u32);
                    *self = PropertyValue::NumberDynamicU32(new_vec);
                }
            }
            PropertyValue::NumberDynamicU32(vec) => {
                if value <= u32::MAX as i128 {
                    vec.push(value as u32);
                } else {
                    let mut new_vec: Vec<u64> = vec.iter().map(|&x| x as u64).collect();
                    new_vec.push(value as u64);
                    *self = PropertyValue::NumberDynamicU64(new_vec);
                }
            }
            PropertyValue::NumberDynamicU64(vec) => {
                if value <= u64::MAX as i128 {
                    vec.push(value as u64);
                } else {
                    let mut new_vec: Vec<i128> = vec.iter().map(|&x| x as i128).collect();
                    new_vec.push(value);
                    *self = PropertyValue::NumberDynamicI128(new_vec);
                }
            }
            PropertyValue::NumberDynamicI8(vec) => {
                if value >= i8::MIN as i128 && value <= i8::MAX as i128 {
                    vec.push(value as i8);
                } else {
                    let mut new_vec: Vec<i16> = vec.iter().map(|&x| x as i16).collect();
                    new_vec.push(value as i16);
                    *self = PropertyValue::NumberDynamicI16(new_vec);
                }
            }
            PropertyValue::NumberDynamicI16(vec) => {
                if value >= i16::MIN as i128 && value <= i16::MAX as i128 {
                    vec.push(value as i16);
                } else {
                    let mut new_vec: Vec<i32> = vec.iter().map(|&x| x as i32).collect();
                    new_vec.push(value as i32);
                    *self = PropertyValue::NumberDynamicI32(new_vec);
                }
            }
            PropertyValue::NumberDynamicI32(vec) => {
                if value >= i32::MIN as i128 && value <= i32::MAX as i128 {
                    vec.push(value as i32);
                } else {
                    let mut new_vec: Vec<i64> = vec.iter().map(|&x| x as i64).collect();
                    new_vec.push(value as i64);
                    *self = PropertyValue::NumberDynamicI64(new_vec);
                }
            }
            PropertyValue::NumberDynamicI64(vec) => {
                if value >= i64::MIN as i128 && value <= i64::MAX as i128 {
                    vec.push(value as i64);
                } else {
                    let mut new_vec: Vec<i128> = vec.iter().map(|&x| x as i128).collect();
                    new_vec.push(value);
                    *self = PropertyValue::NumberDynamicI128(new_vec);
                }
            }
            PropertyValue::NumberDynamicI128(vec) => {
                vec.push(value);
            }
            PropertyValue::NumberDynamicF32(vec) => {
                vec.push(value as f32);
            }
            _ => panic!("Cannot push an integer into non-numeric PropertyType"),
        }
    }

    /// Insert a float value, automatically converts to NumF32
    fn push_float(&mut self, value: f32) {
        match self {
            PropertyValue::NumberDynamicF32(vec) => vec.push(value),
            PropertyValue::NumberDynamicU8(_) | PropertyValue::NumberDynamicU16(_) | PropertyValue::NumberDynamicU32(_) |
            PropertyValue::NumberDynamicU64(_) | PropertyValue::NumberDynamicI16(_) | PropertyValue::NumberDynamicI32(_) |
            PropertyValue::NumberDynamicI64(_) | PropertyValue::NumberDynamicI128(_) => {
                // Promote integer vector to float vector
                let new_vec: Vec<f32> = match self {
                    PropertyValue::NumberDynamicU8(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicU16(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicU32(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicU64(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicI16(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicI32(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicI64(vec) => vec.iter().map(|&x| x as f32).collect(),
                    PropertyValue::NumberDynamicI128(vec) => vec.iter().map(|&x| x as f32).collect(),
                    _ => unreachable!(),
                };
                let mut new_vec = new_vec;
                new_vec.push(value);
                *self = PropertyValue::NumberDynamicF32(new_vec);
            }
            _ => panic!("Cannot push a float into non-numeric PropertyType"),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                        Main                                         ///
///////////////////////////////////////////////////////////////////////////////////////////
fn main() -> io::Result<()> {
    let mut uis;
    timer!("Initialization", {
        uis = Uis::new_from_file(Path::new("example project (FlowForm)/App/Main.uis"))?;

        Component::new(&mut uis, "A", None);
        Component::enter_new(&mut uis, "B", None);
        Component::new(&mut uis, "C", None);
        Component::new(&mut uis, "D", None);
        Component::new(&mut uis, "E", None);
        // uis = Uis::new_from_file(Path::new("tests/basics.uis"))?;

        // println!("{:#?}", uis);        
    });
    println!("{:#?}", uis.components);
    std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}