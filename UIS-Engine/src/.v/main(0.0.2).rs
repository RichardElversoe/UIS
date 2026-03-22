use indexmap::IndexMap;
use std::fmt::Pointer;
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::{env, u128};
use std::rc::Rc;

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
struct Uis {
    path: String,
    data: String, 
    warn: Vec<String>,
    include: IndexMap<String, Uis>,

    properties: IndexMap<String, Property>,
    components: IndexMap<String, Component>,
    states: IndexMap<String, State>,

} impl Uis {
    pub fn new_from_file(source: &str) -> Self {
        let mut uis=  Self {
            path: String::new(),
            data: String::new(),
            warn: Vec::new(),
            include: IndexMap::new(),

            properties: IndexMap::new(),
            components: IndexMap::new(),
            states: IndexMap::new(),
        };

        if let Ok(data) = uis.read_uis_file(source) {
            uis.data = uis.sanitize_data(&data);
        }
        uis.map();

        uis
    }
    pub fn new_from_string(data: String) -> Self {
        let mut uis=  Self {
            path: String::new(),
            data: String::new(),
            warn: Vec::new(),
            include: IndexMap::new(),

            properties: IndexMap::new(),
            components: IndexMap::new(),
            states: IndexMap::new(),
        };

        uis.data = uis.sanitize_data(&data);
        uis.map();

        uis
    }
    fn read_uis_file(&self, source: &str) -> io::Result<String> {
        let mut content = String::new();

        // First, try relative to current working directory
        if let Ok(file) = File::open(source) {
            BufReader::new(file).read_to_string(&mut content)?;
            return Ok(content);
        }

        // Fallback: try relative to executable directory
        if let Ok(exe_dir) = env::current_exe().and_then(|p| Ok(p.parent().unwrap().to_path_buf())) {
            let full_path = exe_dir.join(source);
            if let Ok(file) = File::open(&full_path) {
                BufReader::new(file).read_to_string(&mut content)?;
                content = content.replace("\r\n", "\n").replace("\r", "\n");
                return Ok(content);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not find file '{}'", source),
        ))
    }
    fn sanitize_data(&self, data: &str) -> String {
        //perserve the rough format of the data, to simplify debugging down the line (do not remove "\n")
        let mut new = String::new();
        let mut chars = data.chars().peekable();
        
        let mut in_quote: bool = false;
        let mut comment_encapsualtions: u8 = 0;

        while let Some(ch) = chars.next() {
            if ch == '\n' {
                new.push(ch);
            } else {
                if !in_quote {
                    if ch == '/' {
                        if chars.peek() == Some(&'*') {
                            comment_encapsualtions = comment_encapsualtions.saturating_add(1);
                        } else if chars.peek() == Some(&'/') && comment_encapsualtions == 0 {
                            while let Some(&c) = chars.peek() {
                                if c == '\n' {
                                    break;
                                }
                                chars.next(); // Consume and skip
                            }
                            continue;
                        }
                    } else if ch == '*' {
                        if chars.peek() == Some(&'/') {
                            comment_encapsualtions = comment_encapsualtions.saturating_sub(1);
                            chars.next();
                            if chars.peek() == Some(&'/') {
                                while let Some(&c) = chars.peek() {
                                    if c == '\n' {
                                        break;
                                    }
                                    chars.next(); // Consume and skip
                                }
                                continue;
                            }
                            continue;
                        }
                    }
                }
                if comment_encapsualtions == 0 {
                    if ch == '"' {
                        in_quote = !in_quote;
                    }
                    new.push(ch);
                }
            }
        }

        println!("size in:  {}", data.len());
        println!("size out: {}", new.len());
        // println!("----------\n{}\n----------", new);
        new
    }  
    fn map(&mut self) {
        let mut token = String::new();
        let mut state = tokenState::component;

        let mut is_quote = false;
        let mut is_option = false;
        let mut is_line_start = true;
        let mut continue_at_next_line = false;
        
        let mut addr_depth: usize = 0;
        let mut curl_depth: usize = 0;
        let mut cond_depth: usize = 0;
        let mut paren_depth: usize = 0;

        for ch in self.data.chars() {
            match ch {
                '"' => {
                    is_quote = !is_quote;
                    is_line_start = false;
                }
                _ if is_quote => is_line_start = false,
                '\'' => is_option = !is_option,

                ':' if state == tokenState::component => state = tokenState::property,
                '_' if state == tokenState::component => state = tokenState::state,
                '.' if state != tokenState::property   => state = tokenState::component,

                '{' => { curl_depth = curl_depth.saturating_add(1); is_line_start = false; }
                '}' => { curl_depth = curl_depth.saturating_sub(1); is_line_start = false; }
                '[' => { cond_depth = cond_depth.saturating_add(1); is_line_start = false; }
                ']' => { cond_depth = cond_depth.saturating_sub(1); is_line_start = false; }
                '(' => { paren_depth = paren_depth.saturating_add(1); is_line_start = false; }
                ')' => { paren_depth = paren_depth.saturating_sub(1); is_line_start = false; }

                '\t' if is_line_start => {
                    addr_depth = addr_depth.saturating_add(1);
                    continue;
                }

                ',' | '+' | '-' | '*' | '/' | '|' | '&' => continue_at_next_line = true,

                '\n' => {
                    if is_option {
                        println!("options need to be one line and cant contain spaces or special characters");
                    }
                    if !is_line_start && !continue_at_next_line && cond_depth == 0 && paren_depth == 0 {
                        evaluate_token(&mut addr_depth, &mut token, &mut state);
                        state = tokenState::component;
                    }
                    is_line_start = true;
                    continue_at_next_line = false;
                    continue;
                }

                _ => is_line_start = false,
            }

            continue_at_next_line = false;
            token.push(ch);
        }

        fn evaluate_token(depth: &mut usize, token: &mut String, token_state: &mut tokenState) {
            match token_state {
                tokenState::component =>    print!("Component "),
                tokenState::property =>     print!("property: "),
                tokenState::state =>        print!("_state:   ")
            }
            println!("{}-- {}", depth,token);
            *depth = 0;
            token.clear();
        }
    }
}

#[derive(PartialEq)]
enum tokenState {
    component,
    property,
    state,
}

#[derive(Debug)]
struct Component {
    name: String,
    parent: Option<Rc<Component>>,
    
    ancestors: Option<Vec<Rc<Component>>>,
    decendents: Option<Vec<Rc<Component>>>,

    properties: IndexMap<String, Rc<Property>>,
    components: IndexMap<String, Rc<Component>>,
    states: IndexMap<String, Rc<State>>,
} impl Component {
    pub fn new(name: &str, parent: Option<Rc<Component>>, ) -> Self {
        Self {
            name: name.to_string(),
            parent: parent,
            ancestors: None,
            decendents: None,
            properties: IndexMap::new(),
            components: IndexMap::new(),
            states: IndexMap::new(),
        }
    }
}

#[derive(Debug)]
struct State {
    name: String,
    parent: Option<Rc<Component>>,
    dependent: Option<Rc<State>>,

    expression: Vec<Rc<str>>,
    is_valid: bool,

} impl State {
    
}

#[derive(Debug, Clone)]
struct Property {
    name: String,
    parent: Option<Rc<Component>>,
    dependent: Option<Rc<State>>,

    expression: Vec<Rc<str>>,
    value: Vec<PropertyType>,

} impl Property {
    pub fn new(name: &str, value: &str) -> Self {
        let mut new_property = Self { 
            name: String::new(), 
            parent: None, 
            dependent: None, 
            expression: Vec::new(), 
            value: Vec::new() 
        };

        new_property
    }

    pub fn new_of_type(name: &str, property_type: PropertyType) -> Self {
        let mut new_property = Self { 
            name: String::new(), 
            parent: None, 
            dependent: None, 
            expression: Vec::new(), 
            value: Vec::new() 
        };

        new_property.value = vec![property_type];

        new_property
    }

    fn evaluate_type() -> PropertyType{
        PropertyType::Color(Color { r: 0, g: 0, b: 0, a: 0 })
    }

    pub fn set_value(&self, value: &str){

    }
}

#[derive(Debug, Clone)]
enum PropertyType {
    Number(Number),
    Text(String),
    Color(Color),
    Reference(String),
    Option(String),
    Expression(Rc<str>),
    Structure(String),
} 

#[derive(Debug, Clone, Copy)]
enum Number {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
}

#[derive(Debug, Clone, Copy)]
enum DynamicValue {
    Integer(i128),
    Float(f32),
}
impl From<u8>   for DynamicValue { fn from(v: u8)   -> Self { Self::Integer(v as i128) } }
impl From<u16>  for DynamicValue { fn from(v: u16)  -> Self { Self::Integer(v as i128) } }
impl From<u32>  for DynamicValue { fn from(v: u32)  -> Self { Self::Integer(v as i128) } }
impl From<u64>  for DynamicValue { fn from(v: u64)  -> Self { Self::Integer(v as i128) } }
impl From<i8>   for DynamicValue { fn from(v: i8)   -> Self { Self::Integer(v as i128) } }
impl From<i16>  for DynamicValue { fn from(v: i16)  -> Self { Self::Integer(v as i128) } }
impl From<i32>  for DynamicValue { fn from(v: i32)  -> Self { Self::Integer(v as i128) } }
impl From<i64>  for DynamicValue { fn from(v: i64)  -> Self { Self::Integer(v as i128) } }
impl From<i128> for DynamicValue { fn from(v: i128) -> Self { Self::Integer(v) } }
impl From<f32>  for DynamicValue { fn from(v: f32)  -> Self { Self::Float(v) } }
impl From<f64>  for DynamicValue { fn from(v: f64)  -> Self { Self::Float(v as f32) } }
impl Number {
    pub fn new() -> Self {
        Number::U8(0)
    }

    pub fn insert<T: Into<DynamicValue>>(&mut self, value: T) {
        match value.into() {
            DynamicValue::Float(f) => {
                *self = Number::F32(f);
            }
            DynamicValue::Integer(v) => {
                *self = match self {
                    Number::U8(_)  => Self::promote_unsigned(v, 8),
                    Number::U16(_) => Self::promote_unsigned(v, 16),
                    Number::U32(_) => Self::promote_unsigned(v, 32),
                    Number::U64(_) => Self::promote_unsigned(v, 64),
                    Number::I16(_) => Self::promote_signed(v, 16),
                    Number::I32(_) => Self::promote_signed(v, 32),
                    Number::I64(_) => Self::promote_signed(v, 64),
                    Number::I128(_) => Number::I128(v),
                    Number::F32(_) => Number::F32(v as f32), // already float
                };
            }
        }
    }

    fn promote_unsigned(v: i128, bits: u8) -> Self {
        if v < 0 {
            return Self::promote_signed(v, 16);
        }

        match bits {
            8  => if v <= u8::MAX  as i128 { Self::U8(v as u8) }
                  else { Self::promote_unsigned(v, 16) },
            16 => if v <= u16::MAX as i128 { Self::U16(v as u16) }
                  else { Self::promote_unsigned(v, 32) },
            32 => if v <= u32::MAX as i128 { Self::U32(v as u32) }
                  else { Self::promote_unsigned(v, 64) },
            64 => Self::U64(v as u64),
            _  => Self::I128(v),
        }
    }

    fn promote_signed(v: i128, bits: u8) -> Self {
        match bits {
            16 => if v >= i16::MIN as i128 && v <= i16::MAX as i128 {
                      Self::I16(v as i16)
                  } else {
                      Self::promote_signed(v, 32)
                  },
            32 => if v >= i32::MIN as i128 && v <= i32::MAX as i128 {
                      Self::I32(v as i32)
                  } else {
                      Self::promote_signed(v, 64)
                  },
            64 => if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
                      Self::I64(v as i64)
                  } else {
                      Self::I128(v)
                  },
            _ => Self::I128(v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,

} impl Color {
    pub fn new(value: &str) -> Self{
        let  mut color = Self{r: 0, g: 0, b: 0, a: 0,};
        color = color.set_value("Test");
        color
    }

    pub fn set_value(&mut self, value: &str) -> Self{
        let  mut color = Self{r: 0, g: 0, b: 0, a: 0,};
        color = color.set_color_from_hex("");
        color
    }

    fn set_color_from_hex(&mut self, value: &str) -> Self{
        let mut color = Self{
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        color
    }

    fn set_color_from_rgb(&mut self , value: &str) -> Self{
        let mut color = Self{
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        color
    }
    
}

fn main() -> io::Result<()> {
    let mut uis;
    timer!("Initialization", {
        uis = Uis::new_from_file("tests/basics.uis");
    });

    let mut p_test = Property::new("a", "19");

    uis.properties.insert(p_test.name.to_string(), p_test);

    // std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}