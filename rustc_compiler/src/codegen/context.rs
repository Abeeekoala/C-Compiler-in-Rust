use std::collections::HashMap;
use std::fmt;
use crate::ast::TypeSpecifier;
use crate::error::CompileError;
/// Types of locations where variables can be stored
#[derive(Debug, Clone)]
pub enum StorageLocation {
    Register(String),
    Stack(i32),
    Global(String),
}

/// Symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub location: StorageLocation,
    pub size: usize,
    pub type_info: String,
    pub dimensions: Vec<usize>,
    pub is_pointer: bool,
}

/// Definition of a struct
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub fields: HashMap<String, FieldInfo>,
    pub total_size: usize,
}

/// Struct fields
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub offset: usize,
    pub type_info: String,
    pub size: usize,
}
/// Full type
#[derive(Debug, Clone)]
pub enum FullType {
    Base(TypeSpecifier),
    Pointer(Box<FullType>),
    // Add more variants (e.g., Array) as needed
}
impl fmt::Display for FullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FullType::Base(type_spec) => match type_spec {
                TypeSpecifier::Int => write!(f, "int"),
                TypeSpecifier::Unsigned => write!(f, "unsigned"),
                TypeSpecifier::Char => write!(f, "char"),
                TypeSpecifier::Float => write!(f, "float"),
                TypeSpecifier::Double => write!(f, "double"),
                TypeSpecifier::Void => write!(f, "void"),
                TypeSpecifier::Struct(struct_name) => write!(f, "struct {}", struct_name),
                TypeSpecifier::TypedefName(name) => write!(f, "{}", name), // Will be resolved later
                _ => write!(f, "{:?}", type_spec), // Fallback for unhandled types
            },
            FullType::Pointer(pointee) => write!(f, "{}*", pointee),
        }
    }
}

/// Manages the compilation context
#[derive(Debug)]
pub struct CodeGenContext {
    pub current_function: Option<String>,
    pub symbols: Vec<HashMap<String, Symbol>>,
    pub scope_history: Vec<HashMap<String, Symbol>>,
    pub stack_offset: i32,
    pub label_counter: usize,
    pub output: String,
    pub data_section: String,
    pub in_function: bool,
    pub used_temp_registers: Vec<String>,
    pub available_temp_registers: Vec<String>,
    pub used_fp_registers: Vec<String>,
    pub available_fp_registers: Vec<String>,

    break_labels: Vec<String>,
    continue_labels: Vec<String>,

    pub function_signatures: HashMap<String, String>,
    pub struct_definitions: HashMap<String, StructDefinition>,
    pub typedef_map: HashMap<String, FullType>,
    pub frame_sizes: Vec<usize>,  // Stack of frame sizes for nested functions
}

impl CodeGenContext {
    pub fn new() -> Self {
        let temp_regs = vec![
            "t6", "t5", "t4", "t3", "t2", "t1", "t0",
        ];
        let fp_temp_regs = vec![
            "ft11", "ft10", "ft9", "ft8", "ft7", "ft6", "ft5", "ft4", "ft3", "ft2", "ft1", "ft0",
        ];

        CodeGenContext {
            current_function: None,
            symbols: vec![HashMap::new()],
            scope_history:Vec::new(),
            stack_offset: 0,
            label_counter: 0,
            output: String::new(),
            data_section: String::new(),
            in_function: false,
            used_temp_registers: Vec::new(),
            available_temp_registers: temp_regs.iter().map(|&s| s.to_string()).collect(),
            used_fp_registers: Vec::new(),
            available_fp_registers: fp_temp_regs.iter().map(|&s| s.to_string()).collect(),
            break_labels: Vec::new(),
            continue_labels: Vec::new(),
            function_signatures: HashMap::new(),
            struct_definitions: HashMap::new(),
            typedef_map: HashMap::new(),
            frame_sizes: Vec::new(),
        }
    }

    pub fn emit(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
    }

    pub fn emit_data(&mut self, line: &str) {
        self.data_section.push_str(line);
        self.data_section.push('\n');
    }

    pub fn push_break_label(&mut self, label: String) {
        self.break_labels.push(label);
    }

    pub fn pop_break_label(&mut self) -> Option<String> {
        self.break_labels.pop()
    }

    pub fn get_current_break_label(&self) -> Option<&String> {
        self.break_labels.last()
    }

    pub fn push_continue_label(&mut self, label: String) {
        self.continue_labels.push(label);
    }

    pub fn pop_continue_label(&mut self) -> Option<String> {
        self.continue_labels.pop()
    }

    pub fn get_current_continue_label(&self) -> Option<&String> {
        self.continue_labels.last()
    }

    /// Generate a unique label
    pub fn generate_label(&mut self, prefix: &str) -> String {
        let label = format!(".{}{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    pub fn allocate_register(&mut self) -> Option<String> {
        self.available_temp_registers.pop().map(|reg| {
            self.used_temp_registers.push(reg.clone());
            reg
        })
    }

    /// Free a previously allocated register
    pub fn free_register(&mut self, reg: &str) {
        if let Some(pos) = self.used_temp_registers.iter().position(|r| r == reg) {
            self.used_temp_registers.remove(pos);
            if reg.starts_with('t') && reg.len() == 2 && reg.chars().nth(1).unwrap().is_digit(10) {
                let digit = reg.chars().nth(1).unwrap().to_digit(10).unwrap();
                if digit <= 6 {
                    self.available_temp_registers.push(reg.to_string());
                }
            }
        }
    }

    pub fn get_register(&mut self) -> String {
        // Always allocate a new register, don't reuse existing ones
        let reg = self.allocate_register().unwrap();
        reg
    }

    /// Save all temporary registers before a function call
    pub fn save_temp_registers(&mut self) -> Vec<String> {
        let registers: Vec<_> = self.used_temp_registers.iter().cloned().collect();
        let stack_adjustment = registers.len() * 4; // 4 bytes per register
        self.emit(&format!("    addi sp, sp, -{}", stack_adjustment));
        for (i, reg) in registers.iter().enumerate() {
            let offset = i * 4;
            self.emit(&format!("    sw {}, {}(sp)", reg, offset));
        }
        registers
    }

    /// Restore all temporary registers after a calll to a function
    pub fn restore_temp_registers(&mut self, registers: Vec<String>) {
        let stack_adjustment = registers.len() * 4; // 4 bytes per register
        for (i, reg) in registers.iter().enumerate() {
            let offset = i * 4;
            self.emit(&format!("    lw {}, {}(sp)", reg, offset));
        }
        self.emit(&format!("    addi sp, sp, {}", stack_adjustment));
    }

    pub fn allocate_fp_register(&mut self) -> Option<String> {
        self.available_fp_registers.pop().map(|reg| {
            self.used_fp_registers.push(reg.clone());
            reg
        })
    }

    pub fn free_fp_register(&mut self, reg: &str) {
        if let Some(pos) = self.used_fp_registers.iter().position(|r| r == reg) {
            self.used_fp_registers.remove(pos);

            if reg.starts_with('f') && reg.len() >= 3 &&
               (reg.starts_with("ft") || reg.starts_with("fs")) {
                self.available_fp_registers.push(reg.to_string());
            }
        }
    }

    /// Get float register for temporary use
    pub fn get_fp_register(&mut self) -> String {
        let reg = self.allocate_fp_register().unwrap();
        reg
    }

    /// Save all the temporary floating registers before a function call
    pub fn save_fp_registers(&mut self) -> Vec<String> {
        let registers: Vec<_> = self.used_fp_registers.iter().cloned().collect();
        let stack_adjustment = registers.len() * 4;

        if !registers.is_empty() {
            self.emit(&format!("    addi sp, sp, -{}", stack_adjustment));
            for (i, reg) in registers.iter().enumerate() {
                let offset = i * 4;
                self.emit(&format!("    fsw {}, {}(sp)", reg, offset));
            }
        }
        registers
    }

    /// Restore all temporary float registers after a function call
    pub fn restore_fp_registers(&mut self, registers: Vec<String>) {
        let stack_adjustment = registers.len() * 4;

        if !registers.is_empty() {
            for (i, reg) in registers.iter().enumerate() {
                let offset = i * 4;
                self.emit(&format!("    flw {}, {}(sp)", reg, offset));
            }
            self.emit(&format!("    addi sp, sp, {}", stack_adjustment));
        }
    }


    /// Allocate stack space for variables
    pub fn allocate_stack_space(&mut self, size: usize) -> i32 {
        // Align to 4 bytes
        let aligned_size = (size + 3) & !3;
        self.stack_offset -= aligned_size as i32;
        self.stack_offset
    }

    pub fn enter_scope(&mut self) {
        self.symbols.push(HashMap::new());
        println!("Entering scope, current function: {:?}", self.current_function);
    }

    pub fn exit_scope(&mut self) {
        if self.symbols.len() > 1 {
            let popped_scope = self.symbols.pop().unwrap();
            self.scope_history.push(popped_scope);
        }
    }

    pub fn add_symbol(&mut self, name: &str, symbol: Symbol) {
        self.symbols.last_mut().unwrap().insert(name.to_string(), symbol);
    }

    pub fn add_variable(&mut self, name: String, type_name: String) -> i32 {
        let is_pointer = type_name.contains('*');
        let allocated_size = if type_name == "double" { 8 } else { 4 };
        self.stack_offset -= allocated_size as i32;

        if self.in_function {
            let offset = self.stack_offset;

            self.symbols.last_mut().unwrap().insert(name, Symbol {
                location: StorageLocation::Stack(offset),
                size: allocated_size,
                type_info: type_name,
                dimensions: Vec::new(),
                is_pointer,
            });

            offset
        } else {
            let label = name.clone();
            self.emit_data(&format!("{}:", label));
            self.emit_data(&format!("    .word 0  # Global variable: {}", name));

            self.symbols.last_mut().unwrap().insert(name, Symbol {
                location: StorageLocation::Global(label),
                size: allocated_size,
                type_info: type_name,
                dimensions: Vec::new(),
                is_pointer,
            });
            0
        }
    }

    /// Add an array to the symbol table
    pub fn add_array(&mut self, name: String, type_name: String, dimensions: Vec<usize>) -> i32 {
        let mut total_size = 4;
        for dim in &dimensions {
            total_size *= dim;
        }

        let mut array_type = type_name;
        for dim in &dimensions {
            array_type = format!("{}[{}]", array_type, dim);
        }

        if self.in_function {
            self.stack_offset -= total_size as i32;
            let offset = self.stack_offset;

            self.symbols.last_mut().unwrap().insert(name.clone(), Symbol {
                location: StorageLocation::Stack(offset),
                size: total_size,
                type_info: array_type,
                dimensions: dimensions,
                is_pointer: false,
            });

            offset
        } else {
            let label = name.clone();
            self.emit_data(&format!("{}:", label));
            self.emit_data(&format!("    .space {}  # Global array: {}", total_size, name));

            self.symbols.last_mut().unwrap().insert(name.clone(), Symbol {
                location: StorageLocation::Global(label),
                size: total_size,
                type_info: array_type,
                dimensions: dimensions,
                is_pointer: false,
            });

            0
        }
    }

    /// Looking up the symbol table
    pub fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.symbols.iter().rev() {
        if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    /// Extracting data from the symbol table
    pub fn get_variable(&self, name: &str) -> Option<(i32, String)> {
        for scope in self.symbols.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                match &symbol.location {
                    StorageLocation::Stack(offset) => {
                        return Some((*offset, symbol.type_info.clone()));
                    },
                    StorageLocation::Global(label) => {
                        return Some((0, symbol.type_info.clone()));
                    },
                    _ => {}
                }
            }
        }
        println!("Variable not found: {}", name);
        None
    }

    pub fn generate_function_prologue(&mut self) {
        self.in_function = true;
        let frame_size = self.calculate_frame_size();
        let aligned_frame_size = Self::align_to(frame_size, 16) + 16;
        self.frame_sizes.push(aligned_frame_size);

        self.emit(&format!("    addi sp, sp, -{}", aligned_frame_size));
        self.emit(&format!("    sw ra, {}(sp)", aligned_frame_size - 4));
        self.emit(&format!("    sw s0, {}(sp)", aligned_frame_size - 8));
        self.emit(&format!("    addi s0, sp, {}", aligned_frame_size));
        if aligned_frame_size > 0 {
            self.emit(&format!("    addi sp, sp, -{}", aligned_frame_size));
        }

        self.stack_offset = -(aligned_frame_size as i32);
    }

    /// Generate the function epilogue with proper cleanup
    pub fn generate_function_epilogue(&mut self) {
        let aligned_frame_size = match self.frame_sizes.pop() {
            Some(size) => size,
            None => {
                let frame_size = self.calculate_frame_size();
                Self::align_to(frame_size, 16) + 16
            }
        };

        self.emit("    mv sp, s0");
        // Deallocate local variables
        if aligned_frame_size > 0 {
            self.emit(&format!("    addi sp, sp, -{}", aligned_frame_size));
        }

        self.emit(&format!("    lw s0, {}(sp)", aligned_frame_size - 8));
        self.emit(&format!("    lw ra, {}(sp)", aligned_frame_size - 4));
        self.emit(&format!("    addi sp, sp, {}", aligned_frame_size));
        self.emit("    ret");

        if self.frame_sizes.is_empty() {
            self.in_function = false;
            self.reset_temp_registers();
        }
    }

    pub fn get_assembly(&self) -> String {
        let mut full_assembly = String::new();

        if !self.data_section.is_empty() {
            full_assembly.push_str(".data\n");
            full_assembly.push_str(&self.data_section);
            full_assembly.push_str("\n");
        }

        full_assembly.push_str(".text\n");
        full_assembly.push_str(&self.output);
        full_assembly
    }

    pub fn get_used_registers(&self) -> Vec<String> {
        self.used_temp_registers.clone()
    }

    pub fn reset_temp_registers(&mut self) {
        self.used_temp_registers.clear();

        self.available_temp_registers = vec![
            "t6".to_string(), "t5".to_string(), "t4".to_string(),
            "t3".to_string(), "t2".to_string(), "t1".to_string(), "t0".to_string()
        ];
    }

    pub fn declare_function(&mut self, name: &str, return_type: String, param_types: Vec<String>) {
        self.current_function = Some(name.to_string());
        self.register_function(name, return_type);
    }

    pub fn initialize_global_variable(&mut self, name: String, value: i32) {
        let mut data_lines: Vec<String> = self.data_section.lines().map(String::from).collect();
        let var_declaration = format!("{}:", name);

        let mut i = 0;
        while i < data_lines.len() {
            if data_lines[i].trim() == var_declaration {
                if i + 1 < data_lines.len() {
                    data_lines[i + 1] = format!("    .word {}  # Global variable: {}", value, name);
                }
                break;
            }
            i += 1;
        }
        self.data_section = data_lines.join("\n") + "\n";
    }

    pub fn initialize_global_variable_raw(&mut self, name: String, value: u32) {
        let mut data_lines: Vec<String> = self.data_section.lines().map(String::from).collect();

        let mut i = 0;
        let var_declaration = format!("{}:", name);

        while i < data_lines.len() {
            if data_lines[i].trim() == var_declaration {
                if i + 1 < data_lines.len() {
                    data_lines[i + 1] = format!("    .word 0x{:08x}  # Global variable: {}", value, name);
                }
                break;
            }
            i += 1;
        }

        self.data_section = data_lines.join("\n") + "\n";
    }

    pub fn register_function(&mut self, name: &str, return_type: String) {
        self.function_signatures.insert(name.to_string(), return_type);
    }

    pub fn get_function_return_type(&self, name: &str) -> Option<String> {
        self.function_signatures.get(name).cloned()
    }

    /// Initailising that struct
    pub fn register_struct(&mut self, name: String, fields: Vec<(String, String, usize)>) -> usize {
        let mut struct_fields = HashMap::new();
        let mut current_offset = 0;

        for (field_name, field_type, field_size) in fields {
            current_offset = (current_offset + 3) & !3;
            struct_fields.insert(field_name, FieldInfo {
                offset: current_offset,
                type_info: field_type,
                size: field_size,
            });
            current_offset += field_size;
        }
        let total_size = (current_offset + 3) & !3;
        self.struct_definitions.insert(name, StructDefinition {
            fields: struct_fields,
            total_size,
        });

        total_size
    }

    pub fn get_struct_size(&self, struct_name: &str) -> Option<usize> {
        self.struct_definitions.get(struct_name).map(|def| def.total_size)
    }

    pub fn add_struct_variable(&mut self, var_name: String, struct_name: String) -> i32 {
        let struct_size = self.get_struct_size(&struct_name).unwrap_or(4);
        self.stack_offset -= struct_size as i32;
        let offset = self.stack_offset;
        self.symbols.last_mut().unwrap().insert(var_name, Symbol {
            location: StorageLocation::Stack(offset),
            size: struct_size,
            type_info: format!("struct {}", struct_name),
            dimensions: Vec::new(),
            is_pointer: false,
        });
        offset
    }

    pub fn register_enum(&mut self, name: String, values: Vec<(String, i32)>) {
        let enum_values = values.iter().cloned().collect::<HashMap<_, _>>();
        let mut total_size = 0;

        for (value_name, value) in values {
            total_size += 4;
            let symbol = Symbol {
                location: StorageLocation::Register(format!("#{}", value)),
                size: 4,
                type_info: "int".to_string(),
                dimensions: Vec::new(),
                is_pointer: false,
            };
            self.add_symbol(&value_name, symbol);
        }
    }

    pub fn lookup_enum_value(&self, value_name: &str) -> Option<i32> {
        if let Some(symbol) = self.lookup_symbol(value_name) {
            if let StorageLocation::Register(reg) = &symbol.location {
                if reg.starts_with('#') {
                    if let Ok(value) = reg[1..].parse::<i32>() {
                        return Some(value);
                    }
                }
            }
        }
        None
    }

    fn align_to(size: usize, alignment: usize) -> usize {
        (size + alignment - 1) & !(alignment - 1)
    }

    fn calculate_frame_size(&self) -> usize {
        let locals_size = self.calculate_locals_size();
        let spills_size = self.calculate_register_spills_size();
        locals_size + spills_size
    }

    fn calculate_locals_size(&self) -> usize {
        // If we're not in a function, no locals to count
        if !self.in_function {
            return 0;
        }

        let mut total_size = 0;

        // Iterate through all scopes in the current function
        for scope in &self.symbols {
            for symbol in scope.values() {
                // Only count variables stored on the stack
                if let StorageLocation::Stack(_) = symbol.location {
                    // Add the size of this variable (already aligned)
                    total_size += symbol.size;
                }
            }
        }

        // Return the total, which might need to be aligned at the call site
        total_size
    }

    fn calculate_register_spills_size(&self) -> usize {
        // Estimate space needed for register spills during function calls
        // A simple heuristic could be based on the max number of used registers
        let max_spill_count = self.used_temp_registers.len().max(self.used_fp_registers.len());

        // Each register typically needs 4 bytes (32 bits) on RISC-V
        max_spill_count * 4
    }

    pub fn get_type_size(&self, type_str: &str) -> Result<usize, CompileError> {
        match type_str {
            "int" => Ok(4),
            "float" => Ok(4),
            "double" => Ok(8),
            "char" => Ok(1),
            s if s.starts_with("struct ") => {
                let struct_name = &s["struct ".len()..];
                if let Some(struct_def) = self.struct_definitions.get(struct_name) {
                    let total_size = struct_def.fields.values().map(|field| field.size).sum();
                    Ok(total_size)
                } else {
                    Err(CompileError::CodegenError(format!("Undefined struct: {}", struct_name)))
                }
            },
            _ => {
                if let Some(resolved_type) = self.typedef_map.get(type_str) {
                    self.get_type_size(&resolved_type.to_string())
                } else {
                    Err(CompileError::CodegenError(format!("Unknown type: {}", type_str)))
                }
            }
        }
    }
}
