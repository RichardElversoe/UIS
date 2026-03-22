use indexmap::{IndexMap};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, BufReader, Read};
use std::iter::Peekable;
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
    components: IndexMap<Box<str>,Component>, //A map of all components  
    instances: HashMap<Box<str>, usize>,
    properties: IndexMap<Box<str>,Property>, //A map of all properties    
    expression_order: Vec<ID>,
    hiccups: Vec<Hiccup>,

} impl Uis {
    pub fn new() -> Self{
        Self {
            components: IndexMap::new(),
            instances: HashMap::new(),
            properties: IndexMap::new(),
            expression_order: Vec::new(),
            hiccups: Vec::new(),
        }
    }
    pub fn new_from_file(source: &Path) -> io::Result<Self> {
        let mut content = String::new();

        if let Ok(file) = File::open(source) {
            BufReader::new(file).read_to_string(&mut content)?;
            let uis = Uis::new_from_string(content);
            return Ok(uis);
        }

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
    pub fn new_from_string(data: String) -> Self {
        Uis::new_from_tokens(Token::from_string(data))
    }
    pub fn new_from_tokens(tokens: Vec<Token>) -> Self {
        let mut uis = Uis::new();
        uis.map_tokens(tokens);
        

        uis
    }

    pub fn print_out(&self, components: bool, properties: bool) {
        let mut indent: usize = 0;
        if properties {
            for property in &self.properties {
                if property.1.origin == Origin::Root {
                    // property.1.print_out(self, &mut indent, properties);
                }
            }
        }
        if components {
            for component in &self.components {
                if component.1.origin == Origin::Root {
                    component.1.print_out(self, &mut indent, properties);
                }
            }
        }
    }
    fn map_tokens(&mut self, tokens: Vec<Token>){

        println!("\n\n----------------------------------------------\n\n");
        let mut iter: Peekable<std::slice::Iter<'_, Token>> = tokens.iter().peekable();

        enum AddressState{
            New,
            Derive,
            Modify,
        }
        
        struct AddressLog {
            address: AddressKey,
            indent: u32,
            state: AddressState,
        } impl AddressLog {
            pub fn new_root() -> Self {
                Self { address: AddressKey::Root, indent: 0, state: AddressState::New }
            }
        }
        struct SkipLog {
            first_line: Option<usize>,
            last_line: Option<usize>,
        } impl SkipLog {
            pub fn new(indent: usize) -> Self {
                Self { first_line: None, last_line: None}
            }
            pub fn set(&mut self, indent: usize) {
                match self.first_line {
                    Some(_) => self.last_line = Some(indent),
                    None => self.first_line = Some(indent),
                }
            }
            pub fn conclude(&mut self, uis: &mut Uis) {
                match (self.first_line, self.last_line) {
                    (Some(f), None) => {
                        uis.hiccups.push(Hiccup::Error(Line::At(f), "()".to_string(), "()".to_string()));
                    }
                    (Some(f), Some(l)) => {
                        uis.hiccups.push(Hiccup::Error(Line::Between(f, l), "()".to_string(), "()".to_string()));
                    }
                    _ => ()
                }
                self.first_line = None;
                self.last_line = None;
            }
        }

        let mut current_line_indent: (u32, u32) = (1, 0);
        let mut address_log: Vec<AddressLog> = Vec::new();
        let mut skip_log: SkipLog = SkipLog::new(0);

        while let Some(&token) = iter.peek() {
            iter.next();

            match token {
                Token::NewLineIndent(new_line, new_indent ) => {
                    match iter.peek() {
                        Some(Token::NewLineIndent(l, i)) => {
                            skip(&mut skip_log, &mut iter, (*l, *i)); // ignore empty lines
                        }
                        _ => {
                            if new_indent <= &current_line_indent.1 {
                                current_line_indent.0 = *new_line;
                                current_line_indent.1 = *new_indent;
                                skip_log.conclude(self);
                                
                                address_log.retain(|x| x.indent < *new_indent);

                            } else if *new_indent == current_line_indent.1 + 1 {
                                current_line_indent.0 = *new_line;
                                current_line_indent.1 = *new_indent;
                                skip_log.conclude(self);
                            } else {
                                skip(&mut skip_log, &mut iter, (*new_line, *new_indent));
                            }
                        }
                    }
                    
                }
                Token::Add => {
                    //New
                    match (iter.next(), iter.peek()) {

                        //Component
                        (Some(Token::ComponentKey(key)), Some(Token::NewLineIndent(_, _))) => {
                            let latest_address_indent = address_log.last().unwrap_or(&AddressLog { address: AddressKey::Root, indent: 0, state: AddressState::New });
                            let potential_new_address_indent = Component::new(self, &latest_address_indent.address, key);

                            match potential_new_address_indent {
                                AddressKey::Component(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Component(key), indent: current_line_indent.1, state: AddressState::New });
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }

                        //Property
                        (Some(Token::PropertyKey(key)), Some(Token::Is)) => {
                            let latest_address_indent = address_log.last().unwrap_or(&AddressLog { address: AddressKey::Root, indent: 0, state: AddressState::New});
                            iter.next(); //Collect property

                            let expression: Expression = map_expression(&mut iter, current_line_indent);           
                            
                            match Property::new(self, &latest_address_indent.address, key, Value::Color(None), Some(expression)) {
                                AddressKey::Property(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Property(key), indent: current_line_indent.1, state: AddressState::New});
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }

                        //Skip
                        _ => {
                            skip(&mut skip_log, &mut iter, current_line_indent);
                        }
                    }
                }
                Token::At => {
                    //Modify
                    match (iter.next(), iter.peek()) {
                        (Some(Token::ComponentKey(key) | Token::PropertyKey(key)), Some(Token::NewLineIndent(_, _))) => {
                            match AddressKey::from_string(key) {
                                AddressKey::Component(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Component(key), indent: current_line_indent.1, state: AddressState::Modify });
                                }
                                AddressKey::Property(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Property(key), indent: current_line_indent.1, state: AddressState::New });
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }
                        _ => {
                            skip(&mut skip_log, &mut iter, current_line_indent);
                        },
                    }
                }
                Token::ComponentKey(key) => {
                    let latest_address_indent = address_log.last().unwrap_or(&AddressLog { address: AddressKey::Root, indent: 0, state: AddressState::New });

                    //Derive
                    match (iter.next(), iter.peek()) {
                        //Purely Derived
                        (Some(Token::NewLineIndent(_, _)), _) => {
                            let potential_new_address_indent = Component::derived(self, &latest_address_indent.address, &AddressID::from_string(key, self));

                            match potential_new_address_indent {
                                AddressKey::Component(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Component(key), indent: current_line_indent.1, state: AddressState::New });
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }
                        //Derived with new name
                        (Some(Token::ComponentKey(key)), Some(Token::NewLineIndent(_, _))) => {
                            let potential_new_address_indent = Component::new(self, &latest_address_indent.address, key);

                            match potential_new_address_indent {
                                AddressKey::Component(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Component(key), indent: current_line_indent.1, state: AddressState::New });
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }
                        //Derive with parameters
                        (Some(Token::ComponentKey(key)), Some(Token::Parenthesees(parameters))) => {
                            let potential_new_address_indent = Component::new(self, &latest_address_indent.address, key);

                            match potential_new_address_indent {
                                AddressKey::Component(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Component(key), indent: current_line_indent.1, state: AddressState::New });
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }
                        }
                        _ => {
                            skip(&mut skip_log, &mut iter, current_line_indent);
                        }
                    }
                },
                Token::PropertyKey(key) => {
                    let latest_address_indent = address_log.last().unwrap_or(&AddressLog { address: AddressKey::Root, indent: 0, state: AddressState::New});

                    match (iter.peek(), &latest_address_indent.state) {
                        (Some(Token::Is), AddressState::New) => {
                            iter.next();

                            let expression: Expression = map_expression(&mut iter, current_line_indent);                                                  
                            
                            match Property::new(self, &latest_address_indent.address, key, Value::Color(None), Some(expression)) {
                                AddressKey::Property(key) => {
                                    address_log.push(AddressLog { address: AddressKey::Property(key), indent: current_line_indent.1, state: AddressState::New});
                                }
                                _ => skip(&mut skip_log, &mut iter, current_line_indent),
                            }                                                        
                        }
                        (Some(Token::Is), _) => {
                            iter.next();
                            
                        }
                        _ => {
                            skip(&mut skip_log, &mut iter, current_line_indent);
                        }
                    }
                },
                Token::TypeSpecifier(t) => {
                    skip(&mut skip_log, &mut iter, current_line_indent);
                }
                _ => {
                    skip(&mut skip_log, &mut iter, current_line_indent);
                },
            }
        }
        
        fn skip(skip_log: &mut SkipLog, iter: &mut Peekable<std::slice::Iter<'_, Token>>, current_line_indent: (u32,u32)) {
            skip_log.set(current_line_indent.0 as usize);
            while let Some(&next_token) = iter.peek() {
                match next_token {
                    Token::NewLineIndent(_, i) => {
                        if *i <= current_line_indent.1 {
                            break;
                        }
                        else {
                            iter.next(); // consume
                        }
                    },
                    _ => {
                        iter.next(); // consume
                    },
                }
            }
        }

        fn map_expression(iter: &mut Peekable<std::slice::Iter<'_, Token>>, current_line_indent: (u32,u32)) -> Expression {
            let mut expression: Expression = Expression::new();
            while let Some(&next_token) = iter.peek() {
                iter.next(); // consume
                match next_token {
                    Token::NewLineIndent(_, _) => break,
                    _ => ()
                }
                expression.tokens.push(next_token.clone());
            }  
            expression
        }        
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                        Tokens                                       ///
///////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // Address(AddressKey),
    ComponentKey(String),
    PropertyKey(String),
    NewLineIndent(u32, u32), //Line(left), Indentation(right)
    Color(String),
    Text(String),
    Alias(String),
    NumberPositive(u64),
    NumberNegative(u64), //will be converted to signed once size is determined
    NumberFloat(f64),
    Unit(Unit),
    TypeSpecifier(Type),
    Condition(Vec<Token>),
    Parenthesees(Vec<Token>),

    HAS, //For IDE's (will display all asociated properties and or components of property or component)

    At,
    Is,
    Also,
    Comma,

    Factorial,
    Tetrate,
    SquareRoot,
    Exponent,
    Multiply,
    Divide,
    Add,
    Subtract,

    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    NotEqual,
    Modulo,
    Round,
    RoundUp, //Ceil
    RoundDown, //Floor
    Range,
    All,
    And,
    Or,
    Not,
    Nor,
    Xor,
    Nand,
} impl Token {
    fn from_string(data: String) -> Vec<Token>{
        //sanitize
        let data = data.replace("\r\n", "\n").replace("\r", "\n");
        let mut iter = data.chars().peekable();
        let mut line: u32 = 1;

        let tokens = Token::build_from_iter(&mut iter, &mut line);
        
        for token in &tokens {
            println!("{:?}", &token);
        }

        tokens
    }
    fn build_from_iter(iter: &mut Peekable<std::str::Chars<'_>>, line: &mut u32) -> Vec<Token>{
        let mut tokens: Vec<Token> = Vec::new();
        while let Some(&ch) = iter.peek() {
            match ch {
                '"' => {
                    iter.next(); // consume opening quote
                    let mut text = String::new();

                    while let Some(&c) = iter.peek() {
                        iter.next(); // consume
                        match c {
                            '"' => break,
                            '\n' => *line += 1,
                            _ => ()
                        }
                        text.push(c);
                    }

                    tokens.push(Token::Text(text));
                }
                '?' => {
                    iter.next(); // consume
                    tokens.push(Token::HAS);
                }
                '\'' => {
                    iter.next(); // consume
                    let mut text = String::new();

                    while let Some(&c) = iter.peek() {
                        iter.next(); // consume
                        match c {
                            '\'' => break,
                            '\n' => *line += 1,
                            _ => ()
                        }
                        text.push(c);
                    }

                    tokens.push(Token::Alias(text));
                }
                '(' => {
                    iter.next(); // consume
                    tokens.push(Token::Parenthesees(Token::build_from_iter(iter, line)));
                }
                ')' => {
                    iter.next(); // consume
                    return tokens;
                }
                '[' => {
                    iter.next(); // consume
                    tokens.push(Token::Condition(Token::build_from_iter(iter, line)));
                }
                ']' => {
                    iter.next(); // consume
                    return tokens;
                }
                ':' => {
                    iter.next(); // consume
                    tokens.push(Token::Is);
                }
                ';' => {
                    iter.next(); // consume
                    tokens.push(Token::Also);
                }
                '.' => {
                    iter.next(); // consume
                    let mut dots: u8 = 1;

                    while let Some(&c) = iter.peek() {
                        iter.next(); // consume

                        match c {
                            '.' => {
                                dots += 1;
                                iter.next();
                            },
                            '\n' => *line += 1,
                            _ => break,
                        }
                    }

                    match dots {
                        0 => todo!("Should never be the case -> add propper error handeling"),
                        1 => match tokens.last() {
                            Some(t) => match t {
                                Token::ComponentKey(_) | Token::PropertyKey(_) => (), //Manipulate the component/property key
                                _ => (),
                            }
                            None => (),
                        },
                        2 => tokens.push(Token::Range),
                        3 | _ => tokens.push(Token::All),
                    }
                }
                ',' => {
                    iter.next(); // consume
                    tokens.push(Token::Comma);
                }
                '!' => {
                    iter.next();

                    match iter.peek() {
                        Some(next) => match next {
                            '&' => {
                                iter.next();
                                tokens.push(Token::Nand);
                            }
                            '|' => {
                                iter.next();
                                tokens.push(Token::Nor);
                            }
                            '!' => {
                                iter.next();
                                tokens.push(Token::Factorial);
                            }
                            '=' => {
                                iter.next();
                                tokens.push(Token::NotEqual);
                            }
                            _ => (),
                        },
                        None => tokens.push(Token::Not),
                    }
                }
                '⊻' => {
                    iter.next();
                    tokens.push(Token::Xor);
                }
                '√' => {
                    iter.next();
                    tokens.push(Token::SquareRoot);
                }
                'π' => {
                    iter.next();
                    tokens.push(Token::Unit(Unit::Pi));
                }
                '^' => {
                    iter.next();
                    if iter.peek() == Some(&'^') {
                        iter.next();
                        tokens.push(Token::Tetrate);
                    } else {
                        tokens.push(Token::Exponent);
                    }
                }
                '*' => {
                    iter.next();
                    tokens.push(Token::Multiply);
                }
                '/' => {
                    iter.next();
                    if iter.peek() == Some(&'/') {
                        while let Some(&c) = iter.peek() {
                            if c == '\n' {
                                break;
                            } else {
                                iter.next(); // consume
                            }
                        }
                    } else if iter.peek() == Some(&'*') {
                        while let Some(&c) = iter.peek() {
                            iter.next(); // consume
                            if c == '\n' {
                                *line += 1;
                            } else if c == '*' && iter.peek() == Some(&'/') {
                                break;
                            }
                        }
                    } else {
                        tokens.push(Token::Divide);
                    }
                }
                '+' => {
                    iter.next();
                    tokens.push(Token::Add);
                }
                '-' => {
                    iter.next();
                    tokens.push(Token::Subtract);
                }
                '~' => {
                    iter.next();
                    if iter.peek() == Some(&'>') {
                        iter.next();
                        tokens.push(Token::RoundUp);
                    } else {
                        tokens.push(Token::Round);
                    }
                }
                '<' => {
                    iter.next();

                    match iter.peek() {
                        Some(next) => match next {
                            '~' => {
                                iter.next();
                                tokens.push(Token::RoundDown);
                            }
                            '=' => {
                                iter.next();
                                tokens.push(Token::LessThanOrEqual);
                            }
                            _ => (),
                        },
                        None => tokens.push(Token::LessThan),
                    }
                }
                '>' => {
                    iter.next();
                    if iter.peek() == Some(&'=') {
                        iter.next();
                        tokens.push(Token::GreaterThanOrEqual);
                    } else {
                        tokens.push(Token::GreaterThan);
                    }
                }
                '=' => {
                    iter.next();
                    tokens.push(Token::Equal);
                }
                '&' => {
                    iter.next();
                    tokens.push(Token::And);
                }
                '|' => {
                    iter.next();
                    tokens.push(Token::Or);
                }
                '%' => {
                    iter.next();

                    match iter.peek() {
                        Some(next) => match next {
                            '%' => {
                                iter.next();
                                tokens.push(Token::Modulo);
                            }
                            '|' => {
                                iter.next();
                                tokens.push(Token::Xor);
                            }
                            _ => (),
                        },
                        None => tokens.push(Token::Unit(Unit::Percent)),
                    }
                }
                '#' => {
                    iter.next(); // consume
                    let mut color = String::new();

                    while let Some(&c) = iter.peek() {
                        iter.next(); // consume
                        if c == '\n' {
                            *line += 1;
                        } else if c.is_ascii_hexdigit() {
                            color.push(c);
                            iter.next();
                        } else {
                            break;
                        }
                        color.push(c);
                    }

                    tokens.push(Token::Color(color));
                }
                '0'..='9' => {
                    let mut number = String::new();
                    let mut unit = String::new();
                    let mut is_float = false;

                    while let Some(&c) = iter.peek() {
                        if c.is_ascii_digit() {
                            number.push(c);
                            iter.next();
                        } else if c == '.' && !is_float {
                            is_float = true;
                            number.push(c);
                            iter.next();
                        } else if c == '_' {
                            iter.next(); //Consume underscores (usefulle for anotating larger numbers)
                        } else {
                            break;
                        }
                    }

                    //Push number token
                    if is_float {
                        if let Ok(v) = number.parse::<f64>() {
                            tokens.push(Token::NumberFloat(v));
                        }
                    } else if tokens.last() == Some(&Token::Subtract){
                        if let Ok(v) = number.parse::<u64>() {
                            tokens.push(Token::NumberNegative(v));
                        } 
                    } else {
                        if let Ok(v) = number.parse::<u64>() {
                            tokens.push(Token::NumberPositive(v));
                        }
                    }
                    
                    //Check for unit
                    while let Some(&c) = iter.peek() {
                        if c.is_ascii_alphabetic() {
                            unit.push(c);
                            iter.next();
                        } else {
                            break;
                        }
                    }

                    match Unit::parse(&unit) {
                        Some(u) => tokens.push(Token::Unit(u)),
                        None => (),
                    }

                }
                '@' => {
                    iter.next(); // consume
                    tokens.push(Token::At);
                }
                ' ' => {
                    iter.next(); // consume
                }
                '\n' => {
                    iter.next(); // consume
                    *line += 1;
                    let mut indentations: u32 = 0;
                    while let Some(&c) = iter.peek() {
                        if c == '\t' {
                            indentations += 1;
                            iter.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::NewLineIndent(*line, indentations));
                }
                _ => {
                    iter.next(); // consume unknown
                    if ch.is_alphabetic() {
                        let mut input = String::from(ch);
                        while let Some(&next) = iter.peek() {
                            if next.is_alphanumeric() || next == '.' || next == '(' || next == ')' {
                                input.push(next);
                                iter.next();
                            } else {
                                break;
                            }
                        }
                        match input.bytes().all(|b| (b'A' <= b && b <= b'Z') || b == b'.' || b == b'(' || b == b')') {
                            true => match Type::from_string(&input) {
                                Some(t) => tokens.push(Token::TypeSpecifier(t)),
                                None => match AddressKey::from_string(&input) {
                                    AddressKey::Component(key) => tokens.push(Token::ComponentKey(key)),
                                    AddressKey::Property(key) => tokens.push(Token::PropertyKey(key)),
                                    _ => ()
                                }
                            },
                            false => match AddressKey::from_string(&input) {
                                AddressKey::Component(key) => tokens.push(Token::ComponentKey(key)),
                                AddressKey::Property(key) => tokens.push(Token::PropertyKey(key)),
                                _ => ()
                            }
                        }
                    }
                }
            }
        }
        tokens
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                      Addressing                                     ///
///////////////////////////////////////////////////////////////////////////////////////////

type ID = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum AddressID {
    Component(ID),
    Property(ID),
    Root,
    Invalid,
    
} impl AddressID {
    /**Warning! never use this for new entries - will return invalid for anytning not already initialised*/
    pub fn forward(&self, uis: &Uis, next: &str) -> Self {
        self.to_address_key(uis).forward(uis ,next).to_address_id(uis)
    }
    pub fn back(&self, uis: &Uis, by: u32) -> Self {
        self.to_address_key(uis).back(by).to_address_id(uis)
    }

    pub fn from_string(s: &str, uis: &Uis) -> Self{
        AddressKey::from_string(s).to_address_id(uis)
    }
    pub fn to_address_key(&self, uis: &Uis) -> AddressKey {
        match self {
            AddressID::Component(index) => {
                match uis.components.get_index(*index) {
                    Some(key) => AddressKey::Component(key.0.to_string()),
                    None => AddressKey::Invalid, //ToDo: Add hicckup log
                }
            },
            AddressID::Property(index) => {
                match uis.properties.get_index(*index) {
                    Some(key) => AddressKey::Property(key.0.to_string()),
                    None => AddressKey::Invalid, //ToDo: Add hicckup log
                }
            },
            AddressID::Root => AddressKey::Root,
            AddressID::Invalid => AddressKey::Invalid, //ToDo: Add hicckup log
        }
    }
    pub fn to_optional_string(&self, uis: &Uis) -> Option<String> {
        self.to_address_key(uis).to_optional_string()
    }
    pub fn to_string(&self, uis: &Uis) -> String {
        self.to_address_key(uis).to_string()
    }
    pub fn to_origin(& mut self, uis: &mut Uis) -> Origin{
        self.to_address_key(uis).get_origin(uis)
    }
    pub fn to_component_id(&self) -> Result<ID, Hiccup> {
        match self {
            AddressID::Component(id) => Ok(id.to_owned()),
            _ => Err(Hiccup::Warning(Line::Unknown, "the address is not of type component".to_string(), "".to_string())),
        }
    }

}

#[derive(Debug, Clone, PartialEq)]
pub enum AddressKey{
    Component(String),
    Property(String),
    Root,
    Invalid,
} impl AddressKey {
    pub fn add(&self, uis: &mut Uis, next: &str) -> Self {
        let new: AddressKey;

        match AddressKey::from_string(next) {
            //Components
            AddressKey::Component(n) => match self {
                AddressKey::Component(o) => match n.contains('.') {
                    true => new = AddressKey::Component(format!("{}.({})",o,n)),
                    false => new = AddressKey::Component(format!("{}.{}",o,n)),
                },
                AddressKey::Root => new = AddressKey::Component(format!("{n}")),
                _ => new = AddressKey::Invalid
            },
            //Properties
            AddressKey::Property(n) => match self {
                AddressKey::Component(o) => match n.contains('.') {
                    true => new = AddressKey::Property(format!("{}.({})",o,n)),
                    false => new = AddressKey::Property(format!("{}.{}",o,n)),
                },
                AddressKey::Property(o) => match n.contains('.') {
                    true => new = AddressKey::Property(format!("{}.({})",o,n)),
                    false => new = AddressKey::Property(format!("{}.{}",o,n)),
                },
                AddressKey::Root => new = AddressKey::Property(format!("{n}")),
                _ => new = AddressKey::Invalid
            },
            _ => new = AddressKey::Invalid,
        }

        match &new {
            AddressKey::Component(key) => match uis.components.contains_key(key.as_str()) {
                true => {
                    let key: Box<str> = key.clone().into_boxed_str();
                    match uis.instances.get_mut(&key) {
                        Some(i) => {
                            *i += 1;
                            if key.ends_with(']') {
                                match key.rfind('[') {
                                    Some(end) => {
                                        AddressKey::Component(format!("{key}[{}]", &key[..end]))
                                    }
                                    None => {
                                        AddressKey::Component(format!("{key}[{i}]"))
                                    }
                                }
                                
                            } else {
                                AddressKey::Component(format!("{key}[{i}]"))
                            }
                        }
                        None => {
                            let a: Option<usize> = uis.instances.insert(key.clone(), 1);
                            AddressKey::Component(format!("{key}[1]"))
                        },
                    }
                },
                false => new,
            },
            AddressKey::Property(key) => match uis.properties.contains_key(key.as_str()) {
                true => {
                    uis.hiccups.push(Hiccup::Warning(Line::Unknown, format!("{key} already exists in this component - please remove the '+' or type, from this property"), "the folowing values will be added to the existing property".to_string()));
                    new
                },
                false => new,
            }
            _ => new
        }

    }
    pub fn forward(&self, uis: &Uis, next: &str) -> Self {
        let new: AddressKey;

        match AddressKey::from_string(next) {
            //Components
            AddressKey::Component(n) => match self {
                AddressKey::Component(o) => match n.contains('.') {
                    true => new = AddressKey::Component(format!("{}.({})",o,n)),
                    false => new = AddressKey::Component(format!("{}.{}",o,n)),
                },
                AddressKey::Root => new = AddressKey::Component(format!("{n}")),
                _ => new = AddressKey::Invalid
            },
            //Properties
            AddressKey::Property(n) => match self {
                AddressKey::Component(o) => match n.contains('.') {
                    true => new = AddressKey::Property(format!("{}.({})",o,n)),
                    false => new = AddressKey::Property(format!("{}.{}",o,n)),
                },
                AddressKey::Property(o) => match n.contains('.') {
                    true => new = AddressKey::Property(format!("{}.({})",o,n)),
                    false => new = AddressKey::Property(format!("{}.{}",o,n)),
                },
                AddressKey::Root => new = AddressKey::Property(format!("{n}")),
                _ => new = AddressKey::Invalid
            },
            _ => new = AddressKey::Invalid,
        }

        match &new {
            AddressKey::Component(key) => {
                match uis.components.contains_key(key.as_str()) {
                    true => AddressKey::Component(key.to_string()),
                    false => AddressKey::Invalid
                }
            },
            AddressKey::Property(key) => {
                match uis.properties.contains_key(key.as_str()) {
                    true => AddressKey::Property(key.to_string()),
                    false => AddressKey::Invalid
                }
            },
            AddressKey::Root => AddressKey::Root,
            AddressKey::Invalid => AddressKey::Invalid,
        }
    }
    pub fn back(&self, mut by: u32) -> Self {
        match self {
            AddressKey::Component(key) | AddressKey::Property(key) => {
                let mut iter = key.char_indices().rev().into_iter().peekable();
                let mut closure: usize = 0;
                let mut end_index: usize = 0;
                let mut last_type_is_uppercase = false;

                while let Some(&next_ch) = iter.peek() {
                    iter.next();
                    match next_ch {
                        (_,'(') => {
                            if closure == 0 {
                                return AddressKey::Invalid; //Can't end in open parenthases.
                            } else {
                                closure -= 1;
                            }
                            
                        },
                        (_,')') => {
                            closure += 1;
                        },
                        (i, c) => {
                            if closure == 0 && c == '.' {
                                match by {
                                    0 => (),
                                    1 => {
                                        by -= 1;
                                        if end_index == 0 {
                                            end_index = i;
                                        }
                                    }
                                    _ => {
                                        by -= 1;
                                    }
                                }
                            } else if c.is_alphanumeric() {
                                match (by, iter.peek()) {
                                    (1, Some((_, '.'))) => {
                                        last_type_is_uppercase = c.is_uppercase();
                                    }
                                    _ => continue,
                                    
                                }
                            }
                        }
                    }
                }

                if end_index != 0 {
                    match last_type_is_uppercase {
                        true => {
                            AddressKey::Component(key[..end_index].to_string())
                        },
                        false => {
                            AddressKey::Property(key[..end_index].to_string())
                        },
                    }
                } else {
                    AddressKey::Root
                }
            },
            AddressKey::Root => AddressKey::Root,
            AddressKey::Invalid => AddressKey::Invalid
        }
    }

    pub fn from_string(s: &str) -> Self { 
        //ToDo: Speed up & help sanitize  (A.B.Cdef.... -> A.B.Cdef)
        match s.split('.')
        .filter(|seg| !seg.is_empty())
        .last()
        .and_then(|seg| seg.chars().next()) {
            Some(c) => {
                match c.is_uppercase() {
                    true => AddressKey::Component(s.to_string()),
                    false => AddressKey::Property(s.to_string()),
                }
                
            },
            None => AddressKey::Invalid,
        }
    }
    pub fn to_address_id(&self, uis: &Uis) -> AddressID{
        match self {
            AddressKey::Component(key) => {
                match uis.components.get_index_of(key.as_str()) {
                    Some(index) => AddressID::Component(index),
                    None => AddressID::Invalid
                }
            },
            AddressKey::Property(key) => {
                match uis.properties.get_index_of(key.as_str()) {
                    Some(index) => AddressID::Property(index),
                    None => AddressID::Invalid
                }
            },
            AddressKey::Root => AddressID::Root,
            AddressKey::Invalid => AddressID::Invalid,
        }
    }
    pub fn to_optional_string(&self) -> Option<String> {
        match self {
            AddressKey::Component(s) | AddressKey::Property(s) => Some(s.clone()),
            _ => None,
        }   
    }
    pub fn to_string(&self) -> String {
        match self.to_optional_string() {
            Some(s) => s,
            None => "".to_string()
        }
    }
    pub fn to_component_id(&self, uis: &Uis) -> Result<ID, Hiccup> {
        match self {
            AddressKey::Component(c) => match uis.components.get_index_of(c.as_str()) {
                Some(id) => Ok(id),
                None => Err(Hiccup::Warning(Line::Unknown, "could not find a component with a valid ID".to_string(), "".to_string())),
            },
            _ => Err(Hiccup::Warning(Line::Unknown, "the address is not of type component".to_string(), "".to_string())),
        }
    }
    ///Creates a straight conversion from AddressKey to Origin
    /// 
    ///Use when AddressKey is already at the desired Origin
    pub fn to_origin(&self, uis: &mut Uis) -> Origin {
        match self {
            AddressKey::Component(c) => match uis.components.get_index_of(c.as_str()) {
                Some(id) => Origin::Component(id),
                None => Origin::Invalid,
            },
            AddressKey::Root => Origin::Root,
            AddressKey::Invalid | AddressKey::Property(_) => Origin::Invalid,
        }
    }

    ///Steps back from AddressKey until it finds a valid Origin
    /// 
    ///Use when AddressKey is not at the desired Origin
    pub fn get_origin(&self, uis: &mut Uis) -> Origin {
        match self {
            AddressKey::Invalid => Origin::Invalid,
            AddressKey::Root => Origin::Root,
            AddressKey::Component(key) | AddressKey::Property(key) => {
                let mut reverse_encapsulations: u8 = 0; //lets not be stupid - more should be considered extraordinarily bad practice - handle if/when it becomes an issue
                let mut seperator_found = false;
                let mut index_of_seperation = 0;

                
                for ch in key.char_indices().rev() {
                    match ch.1 {
                        ')' => reverse_encapsulations += 1,
                        '(' => if reverse_encapsulations == 0 {
                                uis.hiccups.push(Hiccup::Error(Line::Unknown, format!("The address: {:#?} ends with opening parenthesees", key), "Causing the origin to return invalid".to_string()));
                                return Origin::Invalid;
                            } else {
                                reverse_encapsulations -= 1;
                            },
                        '.' => match key.chars().nth(ch.0 + 1) {
                            Some(c) => match c.is_uppercase() {
                                true => match seperator_found {
                                    true => match uis.components.get_index_of(&key[..index_of_seperation]) {
                                        Some(id) => {
                                            println!("{}", &key[..index_of_seperation]);
                                            return Origin::Component(id)
                                        },
                                        None => {
                                            return Origin::Invalid
                                        },
                                    },
                                    false => continue,
                                },
                                false => match reverse_encapsulations {
                                    0 => {
                                        seperator_found = true;
                                        index_of_seperation = ch.0;
                                    }
                                    _ => continue,
                                },
                            }
                            None => continue,
                        }
                        _ => continue,
                    }
                }

                if !seperator_found {
                    match key.chars().nth(0) {
                        Some(c) => if c.is_uppercase() {
                            match uis.components.get_index_of(&key[..0]) {
                                Some(id) => return Origin::Component(id),
                                None => return Origin::Invalid,
                            }
                        } else {
                            return Origin::Root;
                        },
                        None => return Origin::Invalid,
                    }
                }

                Origin::Invalid
            },
        }
    }

    pub fn exists(&self, uis: &Uis) -> bool{
        match self {
            AddressKey::Component(key) => uis.components.contains_key(key.as_str()),
            AddressKey::Property(key) => uis.properties.contains_key(key.as_str()),
            AddressKey::Root => true,
            AddressKey::Invalid => false,
        }
    }
    pub fn is_component(&self) -> bool {
        match self {
            AddressKey::Component(_) => true,
            _ => false
        }
    }
    pub fn is_property(&self) -> bool {
        match self {
            AddressKey::Property(_) => true,
            _ => false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Origin {
    Component(ID),
    Root,
    Invalid,
} impl Origin {
    pub fn to_address_id(self) -> AddressID {
        match self {
            Origin::Component(c) => AddressID::Component(c),
            Origin::Root => AddressID::Root,
            Origin::Invalid => AddressID::Invalid,
        }
    }
    pub fn to_address_key(self, uis: &Uis) -> AddressKey {
        match self {
            Origin::Component(index) => match uis.components.get_index(index) {
                Some(c) => AddressKey::Component(c.0.to_string()),
                None => todo!(),
            },
            Origin::Root => AddressKey::Root,
            Origin::Invalid => AddressKey::Invalid,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                        Hiccup                                       ///
///////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum Hiccup {
    //Line, Cause, Effect
    Error(Line, String, String), //Major failures such as failure to create, find or execute a property or component
    Warning(Line, String, String) //Minor failures with fallbacks in place (unexpected but accounted for)
}

#[derive(Debug)]
pub enum Line {
    At(usize),
    Between(usize, usize),
    Unknown
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                       Component                                     ///
///////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Component {
    id: ID, //Component ID
    origin: Origin, //Component ID
    pub ancestor: Option<ID>,
    pub descendants: Vec<ID>,
    pub components: Vec<ID>,
    pub properties: Vec<ID>,

} impl Component {
    pub fn new(uis: &mut Uis, address: &AddressKey, name: &str) -> AddressKey {
        Component::build(uis, address, Some(name), None)
    }
    pub fn derived(uis: &mut Uis, address: &AddressKey, ancestor: &AddressID) -> AddressKey {
        Component::build(uis, address, None, Some(ancestor.clone()))
    }
    pub fn new_derived(uis: &mut Uis, address: &AddressKey, name: &str, ancestor: &AddressID) -> AddressKey {
        Component::build(uis, address, Some(name), Some(ancestor.clone()))
    }
    fn build(uis: &mut Uis, address: &AddressKey, name: Option<&str>, ancestor: Option<AddressID>) -> AddressKey {
        //ToDo: Needs housekeeping...
        let new_address;
        let new_key;
        let origin: Origin;
        let component_name: String;
        let id = uis.components.len();
        let mut ancestor_id: Option<ID> = None;
        
        match name {
            Some(n) => component_name = n.to_string(),
            None => match ancestor {
                Some(a) => match a.to_component_id() {
                    Ok(a_id) => match uis.components.get_index_mut(a_id) {
                        Some(ancestor) => {
                            ancestor.1.descendants.push(id);
                            component_name = ancestor.0.to_string();
                            ancestor_id = Some(a_id);
                        },
                        None => return AddressKey::Invalid,
                    },
                    _ => return AddressKey::Invalid,
                },
                None => return AddressKey::Invalid,
            },
        }
        match address {
            AddressKey::Root => {
                origin = Origin::Root;
                new_address = address.add(uis,&component_name);
            }
            AddressKey::Component(_) => {
                origin = address.to_origin(uis);
                match origin {
                    Origin::Component(o) => match uis.components.get_index_mut(o) {
                        Some(origin_component) => {
                            origin_component.1.components.push(id);
                            new_address = address.add(uis,&component_name);
                        }
                        None => {
                            return AddressKey::Invalid;
                        }
                    }
                    Origin::Root => {
                        new_address = address.add(uis,&component_name);
                    },
                    Origin::Invalid => {
                        uis.hiccups.push(Hiccup::Error(Line::Unknown, format!("Can't Add Component '{component_name}' to '{}' since set address does not exist", address.to_string()), "Will therefore ommit the creation of this component".to_string()));
                        return AddressKey::Invalid;
                    },
                }
            },
            AddressKey::Property(_) => {
                uis.hiccups.push(Hiccup::Error(Line::Unknown, format!("Can't Add Component '{component_name}' inside of Properties"), "Will therefore ommit the creation of this component".to_string()));
                return AddressKey::Invalid;
            },
            AddressKey::Invalid => {
                uis.hiccups.push(Hiccup::Error(Line::Unknown, "Can't Add Component to an invalid Address".to_string(), "Will therefore ommit the creation of this component".to_string()));
                return AddressKey::Invalid;
            },
        }
        match &new_address {
            AddressKey::Component(k) => new_key = k.clone().into_boxed_str(),
            _ => {
                uis.hiccups.push(
                    Hiccup::Error(
                        Line::Unknown, 
                        format!(
                            "'{}' is not a valid name for components, as it returns {:?} {}", 
                            component_name,
                            new_address, 
                            if new_address.is_property() {
                                " due to the last segment of the address not starting in uppercase"
                            } else {
                                ""
                            }
                        ),
                        "Will therefore ommit the creation of this component".to_string()
                    )
                );
                return AddressKey::Invalid
            }
        }
        if uis.components.contains_key(&new_key){
            uis.hiccups.push(
                Hiccup::Error(
                    Line::Unknown, 
                    format!("{:#?} already exists", new_address),
                    "Will therefore ommit the recreation of this component".to_string()
                )
            );
            return AddressKey::Invalid
        }

        let c = Self {
            id: id,
            origin,
            ancestor: ancestor_id,
            descendants: Vec::new(),
            components: Vec::new(),
            properties: Vec::new(),
        };
        uis.components.insert(new_key, c);

        
        
        //Add default properties
        // Property::new(uis, &new_address, "self", Value::ComponentReference(Some(id)));
        // Property::new(uis, &new_address, "parent", Value::ComponentReferences(None));
        // Property::new(uis, &new_address, "siblings", Value::ComponentReferences(None));
        // Property::new(uis, &new_address, "children", Value::ComponentReferences(None));
        
        return new_address;
    }

    pub fn print_out(&self, uis: &Uis, from_indent: &mut usize, with_properties: bool) {
        match uis.components.get_index(self.id) {
            Some(address_component) => {
                dbg!("{}", address_component.0);
                if with_properties {
                    todo!("get properties belonging to component");
                }
            },
            None => println!("could not find component with id {}", self.id),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
///                                       Property                                      ///
///////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Property {
    id: ID,
    origin: Origin,
    value: Value,
    pub expression: Option<Expression>,
    macros: Option<HashMap<Box<str>, Box<str>>>,

} impl Property {
    pub fn new(uis: &mut Uis, address: &AddressKey, name: &str, value: Value, expression: Option<Expression>) -> AddressKey {
        let new_address;
        let new_key;
        let origin: Origin;
        
        match address {
            AddressKey::Root => {
                origin = Origin::Root;
                new_address = address.add(uis,&name);
            }
            AddressKey::Component(_) => {
                origin = address.to_origin(uis);
                new_address = address.add(uis,&name);
            },
            AddressKey::Property(_) => {
                origin = address.get_origin(uis);
                new_address = address.add(uis,&name);
            },
            AddressKey::Invalid => {
                uis.hiccups.push(Hiccup::Error(Line::Unknown, "Can't Add Property to an invalid Address".to_string(), "Will therefore ommit the creation of this component".to_string()));
                return AddressKey::Invalid;
            },
        }

        match &new_address {
            AddressKey::Property(k) => new_key = k,
            _ => {
                uis.hiccups.push(
                    Hiccup::Error(
                        Line::Unknown, format!("'{}' is not a valid name for property, as it returns {:?} {}", name, new_address, 
                            if new_address.is_property() {
                                " due to the last segment of the address not starting in uppercase"
                            } else {""}
                        ), "Will therefore ommit the creation of this component".to_string()
                    )
                );
                return AddressKey::Invalid
            }
        }

        if uis.properties.contains_key(new_key.as_str()){
            uis.hiccups.push(Hiccup::Error(Line::Unknown, format!("{:#?} already exists", new_address), "Will therefore ommit the recreation of this component".to_string()));
            return AddressKey::Invalid
        }

        let p = Self { 
            id: uis.properties.len(), 
            origin: origin, 
            value, 
            expression, 
            macros: None 
        };
        
        uis.properties.insert(new_key.clone().into_boxed_str(), p);

        return new_address; 
    }
    pub fn new_derived(uis: &mut Uis, address: &AddressKey, name: &str, derivative: &AddressKey, value: Value, expression: Option<Expression>) -> AddressKey {
        todo!();
    }

    pub fn print_out(&self, uis: &Uis, from_indent: &mut usize) {
        match uis.properties.get_index(self.id) {
            Some(address_component) => {
                println!("{}", address_component.0);
            },
            None => println!("could not find property with id {}", self.id),
        }
    }

    fn set_expression(mut self, expression: Expression) {
        self.expression = Some(expression);
    }
    fn set_value(value: Value) {

    }
}
#[derive(Debug)]
pub enum Value {
    NumberDynamicU8(Option<Vec<u8>>),
    NumberDynamicU16(Option<Vec<u16>>),
    NumberDynamicU32(Option<Vec<u32>>),
    NumberDynamicU64(Option<Vec<u64>>),
    NumberDynamicI8(Option<Vec<i8>>),
    NumberDynamicI16(Option<Vec<i16>>),
    NumberDynamicI32(Option<Vec<i32>>),
    NumberDynamicI64(Option<Vec<i64>>),
    NumberDynamicI128(Option<Vec<i128>>),
    NumberDynamicF32(Option<Vec<f32>>),
    Text(Option<Vec<String>>),
    Option(Option<Vec<usize>>),
    Color(Option<Vec<(u8,u8,u8,u8)>>),
    ComponentReferences(Option<Vec<ID>>),
    ComponentReference(Option<ID>),
    PropertyReferences(Option<Vec<ID>>),
    PropertyReference(Option<ID>),
    Structure(Option<Vec<ID>>),
} 
// impl Value {
//     /// Insert an integer value, promoting vector type if needed
//     fn push_int(&mut self, value: i128) {
//         match self {
//             Value::NumberDynamicU8(vec) => {
//                 if value <= u8::MAX as i128 {
//                     vec.push(value as u8);
//                 } else {
//                     let mut new_vec: Vec<u16> = vec.iter().map(|&x| x as u16).collect();
//                     new_vec.push(value as u16);
//                     *self = Value::NumberDynamicU16(new_vec);
//                 }
//             }
//             Value::NumberDynamicU16(vec) => {
//                 if value <= u16::MAX as i128 {
//                     vec.push(value as u16);
//                 } else {
//                     let mut new_vec: Vec<u32> = vec.iter().map(|&x| x as u32).collect();
//                     new_vec.push(value as u32);
//                     *self = Value::NumberDynamicU32(new_vec);
//                 }
//             }
//             Value::NumberDynamicU32(vec) => {
//                 if value <= u32::MAX as i128 {
//                     vec.push(value as u32);
//                 } else {
//                     let mut new_vec: Vec<u64> = vec.iter().map(|&x| x as u64).collect();
//                     new_vec.push(value as u64);
//                     *self = Value::NumberDynamicU64(new_vec);
//                 }
//             }
//             Value::NumberDynamicU64(vec) => {
//                 if value <= u64::MAX as i128 {
//                     vec.push(value as u64);
//                 } else {
//                     let mut new_vec: Vec<i128> = vec.iter().map(|&x| x as i128).collect();
//                     new_vec.push(value);
//                     *self = Value::NumberDynamicI128(new_vec);
//                 }
//             }
//             Value::NumberDynamicI8(vec) => {
//                 if value >= i8::MIN as i128 && value <= i8::MAX as i128 {
//                     vec.push(value as i8);
//                 } else {
//                     let mut new_vec: Vec<i16> = vec.iter().map(|&x| x as i16).collect();
//                     new_vec.push(value as i16);
//                     *self = Value::NumberDynamicI16(new_vec);
//                 }
//             }
//             Value::NumberDynamicI16(vec) => {
//                 if value >= i16::MIN as i128 && value <= i16::MAX as i128 {
//                     vec.push(value as i16);
//                 } else {
//                     let mut new_vec: Vec<i32> = vec.iter().map(|&x| x as i32).collect();
//                     new_vec.push(value as i32);
//                     *self = Value::NumberDynamicI32(new_vec);
//                 }
//             }
//             Value::NumberDynamicI32(vec) => {
//                 if value >= i32::MIN as i128 && value <= i32::MAX as i128 {
//                     vec.push(value as i32);
//                 } else {
//                     let mut new_vec: Vec<i64> = vec.iter().map(|&x| x as i64).collect();
//                     new_vec.push(value as i64);
//                     *self = Value::NumberDynamicI64(new_vec);
//                 }
//             }
//             Value::NumberDynamicI64(vec) => {
//                 if value >= i64::MIN as i128 && value <= i64::MAX as i128 {
//                     vec.push(value as i64);
//                 } else {
//                     let mut new_vec: Vec<i128> = vec.iter().map(|&x| x as i128).collect();
//                     new_vec.push(value);
//                     *self = Value::NumberDynamicI128(new_vec);
//                 }
//             }
//             Value::NumberDynamicI128(vec) => {
//                 vec.push(value);
//             }
//             Value::NumberDynamicF32(vec) => {
//                 vec.push(value as f32);
//             }
//             _ => panic!("Cannot push an integer into non-numeric PropertyType"),
//         }
//     }

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    metadata: u8,
    pub tokens: Vec<Token>,
} impl Expression {
    pub fn new() -> Self {
        Self {
            metadata: 0,
            tokens: Vec::new(),
        }
    }
}
//     /// Insert a float value, automatically converts to NumF32
//     fn push_float(&mut self, value: f32) {
//         match self {
//             Value::NumberDynamicF32(vec) => vec.push(value),
//             Value::NumberDynamicU8(_) | Value::NumberDynamicU16(_) | Value::NumberDynamicU32(_) |
//             Value::NumberDynamicU64(_) | Value::NumberDynamicI16(_) | Value::NumberDynamicI32(_) |
//             Value::NumberDynamicI64(_) | Value::NumberDynamicI128(_) => {
//                 // Promote integer vector to float vector
//                 let new_vec: Vec<f32> = match self {
//                     Value::NumberDynamicU8(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicU16(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicU32(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicU64(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicI16(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicI32(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicI64(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     Value::NumberDynamicI128(vec) => vec.iter().map(|&x| x as f32).collect(),
//                     _ => unreachable!(),
//                 };
//                 let mut new_vec = new_vec;
//                 new_vec.push(value);
//                 *self = Value::NumberDynamicF32(new_vec);
//             }
//             _ => panic!("Cannot push a float into non-numeric PropertyType"),
//         }
//     }
// }

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum Unit {
    // =========================
    // Length
    // =========================
    Yoctometer,
    Zeptometer,
    Attometer,
    Femtometer,
    Picometer,
    Nanometer,
    Micrometer,
    Millimeter,
    Centimeter,
    Decimeter,
    Meter,
    Decameter,
    Hectometer,
    Kilometer,
    Megameter,
    Gigameter,
    Terameter,
    Petameter,
    Exameter,
    Zettameter,
    Yottameter,

    Inch,
    Foot,
    Yard,
    Mile,

    // =========================
    // Time
    // =========================
    Yoctosecond,
    Zeptosecond,
    Attosecond,
    Femtosecond,
    Picosecond,
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Year,
    Decade,

    // =========================
    // Mass
    // =========================
    Yoctogram,
    Zeptogram,
    Attogram,
    Femtogram,
    Picogram,
    Nanogram,
    Microgram,
    Milligram,
    Gram,
    Kilogram,
    Megagram,
    Gigagram,
    Tonne,

    Ounce,
    Pound,
    Stone,
    ShortTon,
    LongTon,

    // =========================
    // Electric current
    // =========================
    Yoctoampere,
    Zeptoampere,
    Attoampere,
    Femtoampere,
    Picoampere,
    Nanoampere,
    Microampere,
    Milliampere,
    Ampere,
    Kiloampere,
    Megaampere,

    // =========================
    // Temperature
    // =========================
    Kelvin,
    Celsius,
    Fahrenheit,

    // =========================
    // Amount of substance — SI
    // =========================
    Mole,

    // =========================
    // Luminous intensity — SI
    // =========================
    Candela,

    // =========================
    // Volume
    // =========================
    Yoctoliter,
    Zeptoliter,
    Attoliter,
    Femtoliter,
    Picoliter,
    Nanoliter,
    Microliter,
    Milliliter,
    Liter,
    Kiloliter,
    Megaliter,

    FluidOunce,
    Pint,
    Quart,
    Gallon,


    // =========================
    // Additional units (common)
    // =========================
    Percent,
    Pixel,
    Pi,

    // =========================
    // Derived SI units (common)
    // =========================
    Hertz,
    Newton,
    Pascal,
    Joule,
    Watt,
    Coulomb,
    Volt,
    Farad,
    Ohm,
    Siemens,
    Weber,
    Tesla,
    Henry,
    Lumen,
    Lux,
    Becquerel,
    Gray,
    Sievert,
    Katal,
} impl Unit {
    pub fn parse(input: &str) -> Option<Unit> {
        match input {
            // =========================
            // Length
            // =========================
            "ym" | "yoctometer" | "yoctometers" => Some(Unit::Yoctometer),
            "zm" | "zeptometer" | "zeptometers" => Some(Unit::Zeptometer),
            "am" | "attometer" | "attometers" => Some(Unit::Attometer),
            "fm" | "femtometer" | "femtometers" => Some(Unit::Femtometer),
            "pm" | "picometer" | "picometers" => Some(Unit::Picometer),
            "nm" | "nanometer" | "nanometers" => Some(Unit::Nanometer),
            "µm" | "um" | "micrometer" | "micrometers" => Some(Unit::Micrometer),
            "mm" | "millimeter" | "millimeters" => Some(Unit::Millimeter),
            "cm" | "centimeter" | "centimeters" => Some(Unit::Centimeter),
            "dm" | "decimeter" | "decimeters" => Some(Unit::Decimeter),
            "m"  | "meter" | "meters" => Some(Unit::Meter),
            "dam" | "decameter" | "decameters" => Some(Unit::Decameter),
            "hm" | "hectometer" | "hectometers" => Some(Unit::Hectometer),
            "km" | "kilometer" | "kilometers" => Some(Unit::Kilometer),
            "Mm" | "megameter" | "megameters" => Some(Unit::Megameter),
            "Gm" | "gigameter" | "gigameters" => Some(Unit::Gigameter),
            "Tm" | "terameter" | "terameters" => Some(Unit::Terameter),
            "Pm" | "petameter" | "petameters" => Some(Unit::Petameter),
            "Em" | "exameter" | "exameters" => Some(Unit::Exameter),
            "Zm" | "zettameter" | "zettameters" => Some(Unit::Zettameter),
            "Ym" | "yottameter" | "yottameters" => Some(Unit::Yottameter),

            "in" | "inch" | "inches" => Some(Unit::Inch),
            "ft" | "foot" | "feet" => Some(Unit::Foot),
            "yd" | "yard" | "yards" => Some(Unit::Yard),
            "mi" | "mile" | "miles" => Some(Unit::Mile),

            // =========================
            // Time
            // =========================
            "ys" | "yoctosecond" => Some(Unit::Yoctosecond),
            "zs" | "zeptosecond" => Some(Unit::Zeptosecond),
            "as" | "attosecond" => Some(Unit::Attosecond),
            "fs" | "femtosecond" => Some(Unit::Femtosecond),
            "ps" | "picosecond" => Some(Unit::Picosecond),
            "ns" | "nanosecond" => Some(Unit::Nanosecond),
            "µs" | "us" | "microsecond" => Some(Unit::Microsecond),
            "ms" | "millisecond" => Some(Unit::Millisecond),
            "s" | "sec" | "second" | "seconds" => Some(Unit::Second),
            "min" | "minute" | "minutes" => Some(Unit::Minute),
            "h" | "hr" | "hour" | "hours" => Some(Unit::Hour),
            "d" | "day" | "days" => Some(Unit::Day),
            "wk" | "week" | "weeks" => Some(Unit::Week),
            "yr" | "year" | "years" => Some(Unit::Year),
            "decade" | "decades" => Some(Unit::Decade),

            // =========================
            // Mass
            // =========================
            "g" | "gram" | "grams" => Some(Unit::Gram),
            "kg" | "kilogram" | "kilograms" => Some(Unit::Kilogram),
            "mg" | "milligram" | "milligrams" => Some(Unit::Milligram),
            "µg" | "ug" | "microgram" => Some(Unit::Microgram),
            "t" | "tonne" | "tonnes" => Some(Unit::Tonne),

            "oz" | "ounce" | "ounces" => Some(Unit::Ounce),
            "lb" | "lbs" | "pound" | "pounds" => Some(Unit::Pound),
            "st" | "stone" | "stones" => Some(Unit::Stone),
            "shortton" | "short ton" => Some(Unit::ShortTon),
            "longton" | "long ton" => Some(Unit::LongTon),

            // =========================
            // Temperature
            // =========================
            "k" | "kelvin" => Some(Unit::Kelvin),
            "c" | "°c" | "celsius" => Some(Unit::Celsius),
            "f" | "°f" | "fahrenheit" => Some(Unit::Fahrenheit),

            // =========================
            // Amount of substance
            // =========================
            "mol" | "mole" | "moles" => Some(Unit::Mole),

            // =========================
            // Luminous intensity
            // =========================
            "cd" | "candela" => Some(Unit::Candela),

            // =========================
            // Additional
            // =========================
            "%" | "pct" | "percent" => Some(Unit::Percent),
            "px" | "pixel" | "pixels" => Some(Unit::Pixel),
            "pi" | "π" => Some(Unit::Pi),

            // =========================
            // Derived
            // =========================
            "hz" | "hertz" => Some(Unit::Hertz),
            "n" | "newton" => Some(Unit::Newton),
            "pa" | "pascal" => Some(Unit::Pascal),
            "j" | "joule" => Some(Unit::Joule),
            "w" | "watt" => Some(Unit::Watt),
            "v" | "volt" => Some(Unit::Volt),
            "ohm" | "Ω" => Some(Unit::Ohm),

            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Number, //Fully dynamic
    NumberUnsigned, //Semi dynamic
    NumberSigned, //Semi dynamic
    NumberFloat, //Semi dynamic
    NumberU8, //Static
    NumberU16, //Static 
    NumberU32, //Static
    NumberU64, //Static
    NumberI8, //Static
    NumberI16, //Static
    NumberI32, //Static
    NumberI64, //Static
    NumberF32, //Static
    NumberF64, //Static
    Text,
    Option,
    Color,
    ComponentReference,
    PropertyReference,
    Structure,
} impl Type {
    fn from_string(input: &str) -> Option<Self> {
        match input {
            "NUMBER" => Some(Type::Number),
            "NUMBER.UNSIGNED" => Some(Type::NumberUnsigned),
            "NUMBER.UNSIGNED.8" => Some(Type::NumberU8),
            "NUMBER.UNSIGNED.16" => Some(Type::NumberU16),
            "NUMBER.UNSIGNED.32" => Some(Type::NumberU32),
            "NUMBER.UNSIGNED.64" => Some(Type::NumberU64),
            "NUMBER.SIGNED" => Some(Type::NumberSigned),
            "NUMBER.SIGNED.8" => Some(Type::NumberI8),
            "NUMBER.SIGNED.16" => Some(Type::NumberI16),
            "NUMBER.SIGNED.32" => Some(Type::NumberI32),
            "NUMBER.SIGNED.64" => Some(Type::NumberI64),
            "NUMBER.FLOATINGPOINT" => Some(Type::NumberFloat),
            "NUMBER.FLOATINGPOINT.32" => Some(Type::NumberF32),
            "NUMBER.FLOATINGPOINT.64" => Some(Type::NumberF64),
            "TEXT" => Some(Type::Text),
            "OPTION" => Some(Type::Option),
            "COLOR" => Some(Type::Color),
            "COMPONENT" => Some(Type::ComponentReference),
            "PROPERTY" => Some(Type::PropertyReference),
            "STRUCTURE" => Some(Type::Structure),
            _ => None
        }
    }
    pub fn from_tokens(input: Vec<Token>) -> Self {
        Type::Color
    }
}


///////////////////////////////////////////////////////////////////////////////////////////
///                                        Main                                         ///
///////////////////////////////////////////////////////////////////////////////////////////
fn main() -> io::Result<()> {
    let mut uis;
    timer!("Initialization", {
        // uis = Uis::new_from_file(Path::new("example project (FlowForm)/App/Main.uis"))?;
        uis = Uis::new_from_file(Path::new("tests/test.uis"))?;

        // let a = Component::new(&mut uis, &AddressKey::Root, "Hi");
        // let b = Component::new(&mut uis, &AddressKey::Root, "Hello");
        // let bb = b.to_address_id(&uis);
        // let c = Component::derived(&mut uis, &a, &bb);
        // let c = Component::new_derived(&mut uis, &a, "Hej" ,&bb);
        // Property::new(&mut uis, &c, "test", Value::None);

        // println!("\n----------------------------------------------------------------------------------------------\n\n{:#?}", uis);
        
        uis.print_out(true, true );
    });
    // println!("{:#?}\n----------------------------------------------------------", uis.components);
    // println!("{:#?}\n----------------------------------------------------------", uis.properties);
    // println!("{:#?}", uis.hiccups);
    std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}