use std::collections::HashMap;

/// Types of locations where variables can be stored
#[derive(Debug, Clone)]
pub enum StorageLocation {
    /// Register allocation
    Register(String),
    /// Stack allocation with offset from frame pointer
    Stack(i32),
    /// Global variable in data section
    Global(String),
}

/// Symbol table entry for tracking variables
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Storage location for this symbol
    pub location: StorageLocation,
    /// Size in bytes
    pub size: usize,
    /// Type information (basic for now)
    pub type_info: String,
    /// Dimensions of the array
    pub dimensions: Vec<usize>,
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

/// Manages the compilation context
#[derive(Debug)]
pub struct CodeGenContext {
    /// Current function name
    pub current_function: Option<String>,
    /// Symbol table with scopes
    pub symbols: Vec<HashMap<String, Symbol>>,
    pub scope_history: Vec<HashMap<String, Symbol>>,
    /// Next available stack offset
    pub stack_offset: i32,
    /// Next available label number for generating unique labels
    pub label_counter: usize,
    /// Assembly output
    pub output: String,
    pub data_section: String,
    /// Flag to indicate whether we're inside a function
    pub in_function: bool,
    /// Temporary registers currently in use
    pub used_temp_registers: Vec<String>,
    /// Available temporary registers
    pub available_temp_registers: Vec<String>,
    pub used_fp_registers: Vec<String>,
    pub available_fp_registers: Vec<String>,

    break_labels: Vec<String>,
    continue_labels: Vec<String>,

    /// Map of function names to their return types
    pub function_signatures: HashMap<String, String>,

    pub struct_definitions: HashMap<String, StructDefinition>,
}

impl CodeGenContext {
    /// Create a new code generation context
    pub fn new() -> Self {
        // Initialize with RISC-V calling convention registers
        let temp_regs = vec![
            "t6", "t5", "t4", "t3", "t2", "t1", "t0",
        ];

        let fp_temp_regs = vec![
            "ft11", "ft10", "ft9", "ft8", "ft7", "ft6", "ft5", "ft4", "ft3", "ft2", "ft1", "ft0",
        ];

        CodeGenContext {
            current_function: None,
            symbols: vec![HashMap::new()], // Initialize with global scope
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
        }
    }

    /// Add a line of assembly to the output
    pub fn emit(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
    }

    /// Add a line to the data section
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

    /// Integer Register methods:
    /// Allocate a register for temporary use
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

            // Only put registers back in the available pool if they're from our predefined list
            // (t0-t6) - Dynamically allocated registers aren't reused
            if reg.starts_with('t') && reg.len() == 2 && reg.chars().nth(1).unwrap().is_digit(10) {
                let digit = reg.chars().nth(1).unwrap().to_digit(10).unwrap();
                if digit <= 6 {  // Only t0-t6 are predefined
                    self.available_temp_registers.push(reg.to_string());
                }
            }
        }
    }

    /// Get a register for temporary use
    pub fn get_register(&mut self) -> String {
        // Always allocate a new register, don't reuse existing ones
        let reg = self.allocate_register().unwrap();
        reg
    }

    /// Save all temporary registers before a function call
    pub fn save_temp_registers(&mut self) -> Vec<String> {
        // Collect register info first to avoid borrow checker issues
        let registers: Vec<_> = self.used_temp_registers.iter().cloned().collect();
        let stack_adjustment = registers.len() * 4; // 4 bytes per register
        self.emit(&format!("    addi sp, sp, -{}", stack_adjustment));
        for (i, reg) in registers.iter().enumerate() {
            let offset = i * 4;
            self.emit(&format!("    sw {}, {}(sp)", reg, offset));
        }
        registers
    }

    /// Restore all temporary registers after a function call
    pub fn restore_temp_registers(&mut self, registers: Vec<String>) {
        let stack_adjustment = registers.len() * 4; // 4 bytes per register
        for (i, reg) in registers.iter().enumerate() {
            let offset = i * 4;
            self.emit(&format!("    lw {}, {}(sp)", reg, offset));
        }
        self.emit(&format!("    addi sp, sp, {}", stack_adjustment));
    }

    /// Floating-point Register methods:
    /// Allocate a floating-point register for temporary use
    pub fn allocate_fp_register(&mut self) -> Option<String> {
        self.available_fp_registers.pop().map(|reg| {
            self.used_fp_registers.push(reg.clone());
            reg
        })
    }

    /// Free a previously allocated floating-point register
    pub fn free_fp_register(&mut self, reg: &str) {
        if let Some(pos) = self.used_fp_registers.iter().position(|r| r == reg) {
            self.used_fp_registers.remove(pos);

            // Only put registers back in the available pool if they're from our predefined list
            if reg.starts_with('f') && reg.len() >= 3 &&
               (reg.starts_with("ft") || reg.starts_with("fs")) {
                self.available_fp_registers.push(reg.to_string());
            }
        }
    }

    /// Get a floating-point register for temporary use
    pub fn get_fp_register(&mut self) -> String {
        let reg = self.allocate_fp_register().unwrap();
        reg
    }

    /// Save all temporary floating-point registers before a function call
    pub fn save_fp_registers(&mut self) -> Vec<String> {
        // Collect register info first to avoid borrow checker issues
        let registers: Vec<_> = self.used_fp_registers.iter().cloned().collect();
        let stack_adjustment = registers.len() * 4; // 4 bytes per register (for single precision)

        if !registers.is_empty() {
            self.emit(&format!("    addi sp, sp, -{}", stack_adjustment));
            for (i, reg) in registers.iter().enumerate() {
                let offset = i * 4;
                self.emit(&format!("    fsw {}, {}(sp)", reg, offset));
            }
        }

        registers
    }

    /// Restore all temporary floating-point registers after a function call
    pub fn restore_fp_registers(&mut self, registers: Vec<String>) {
        let stack_adjustment = registers.len() * 4; // 4 bytes per register

        if !registers.is_empty() {
            for (i, reg) in registers.iter().enumerate() {
                let offset = i * 4;
                self.emit(&format!("    flw {}, {}(sp)", reg, offset));
            }
            self.emit(&format!("    addi sp, sp, {}", stack_adjustment));
        }
    }


    /// Allocate space on the stack for a variable
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
        if self.symbols.len() > 1 { // Preserve global scope
            let popped_scope = self.symbols.pop().unwrap();
            self.scope_history.push(popped_scope);
        }
    }
    /// Add a symbol to the symbol table
    pub fn add_symbol(&mut self, name: &str, symbol: Symbol) {
        self.symbols.last_mut().unwrap().insert(name.to_string(), symbol);
    }

    /// Add a variable to the symbol table
    pub fn add_variable(&mut self, name: String, type_name: String) -> i32 {
        let allocated_size = if type_name == "double" { 8 } else { 4 };
        self.stack_offset -= allocated_size as i32;

        if self.in_function {
            // Local variable - allocate on stack
            let offset = self.stack_offset;

            self.symbols.last_mut().unwrap().insert(name, Symbol {
                location: StorageLocation::Stack(offset),
                size: allocated_size,
                type_info: type_name,
                dimensions: Vec::new(),
            });

            offset
        } else {
            // Global variable - add to data section
            let label = name.clone();
            self.emit_data(&format!("{}:", label));

            // Check if there's an initializer value from the parser
            // For now, we just initialize to 0
            self.emit_data(&format!("    .word 0  # Global variable: {}", name));

            self.symbols.last_mut().unwrap().insert(name, Symbol {
                location: StorageLocation::Global(label),
                size: allocated_size,
                type_info: type_name,
                dimensions: Vec::new(),
            });

            0
        }
    }

    /// Add an array to the symbol table
    pub fn add_array(&mut self, name: String, type_name: String, dimensions: Vec<usize>) -> i32 {
        // Calculate total array size in bytes
        let mut total_size = 4; // Base element size (int = 4 bytes)
        for dim in &dimensions {
            total_size *= dim;
        }

        // Create array type representation
        let mut array_type = type_name;
        for dim in &dimensions {
            array_type = format!("{}[{}]", array_type, dim);
        }

        if self.in_function {
            // Local array - allocate on stack
            self.stack_offset -= total_size as i32;
            let offset = self.stack_offset;

            // Add to symbols HashMap
            self.symbols.last_mut().unwrap().insert(name.clone(), Symbol {
                location: StorageLocation::Stack(offset),
                size: total_size,
                type_info: array_type,
                dimensions: dimensions,
            });

            offset
        } else {
            // Global array - add to data section
            let label = name.clone();
            self.emit_data(&format!("{}:", label));

            // For uninitialized array, reserve space
            self.emit_data(&format!("    .space {}  # Global array: {}", total_size, name));

            // Add to symbols HashMap
            self.symbols.last_mut().unwrap().insert(name.clone(), Symbol {
                location: StorageLocation::Global(label),
                size: total_size,
                type_info: array_type,
                dimensions: dimensions,
            });

            0 // Return value doesn't matter for globals
        }
    }

    /// Look up a symbol in the symbol table
    pub fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.symbols.iter().rev() {
        if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    /// Get a variable's offset and type from the symbol table
    pub fn get_variable(&self, name: &str) -> Option<(i32, String)> {
        // Flatten the nested options to return just one option
        for scope in self.symbols.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                match &symbol.location {
                    StorageLocation::Stack(offset) => {
                        return Some((*offset, symbol.type_info.clone()));
                    },
                    StorageLocation::Global(label) => {
                        // For global variables, return 0 as the offset
                        // The actual label will be handled elsewhere
                        return Some((0, symbol.type_info.clone()));
                    },
                    _ => {} // Ignore other storage locations
                }
            }
        }
        println!("Variable not found: {}", name);
        None
    }

    /// Generate the function prologue with support for recursion
    pub fn generate_function_prologue(&mut self) {
        self.in_function = true;

        // Save the old stack frame
        self.emit("    addi sp, sp, -32");      // Make space for saved registers
        self.emit("    sw ra, 28(sp)");         // Save return address
        self.emit("    sw s0, 24(sp)");         // Save old frame pointer
        self.emit("    addi s0, sp, 32");       // Set up new frame pointer

        // Save callee-saved registers we'll use
        // self.emit("    sw s1, 20(sp)");
        // self.emit("    sw s2, 16(sp)");

        // Reserve space for local variables (aligned to 16 bytes)
        self.emit("    addi sp, sp, -32");      // Initial space for locals

        self.stack_offset = -32;                 // Track stack allocations from here
    }

    /// Generate the function epilogue with proper cleanup
    pub fn generate_function_epilogue(&mut self) {
        // Deallocate local variables
        self.emit("    mv sp, s0");             // Restore stack pointer
        self.emit("    addi sp, sp, -32");      // Point to saved registers

        // Restore saved registers
        // self.emit("    lw s2, 16(sp)");
        // self.emit("    lw s1, 20(sp)");
        self.emit("    lw s0, 24(sp)");
        self.emit("    lw ra, 28(sp)");

        // Restore stack and return
        self.emit("    addi sp, sp, 32");
        self.emit("    ret");

        self.in_function = false;
        self.reset_temp_registers();
    }

    /// Get the generated assembly code (combined data and text sections)
    pub fn get_assembly(&self) -> String {
        let mut full_assembly = String::new();

        // Add data section if it's not empty
        if !self.data_section.is_empty() {
            full_assembly.push_str(".data\n");
            full_assembly.push_str(&self.data_section);
            full_assembly.push_str("\n");
        }

        // Add text section
        full_assembly.push_str(".text\n");
        full_assembly.push_str(&self.output);

        full_assembly
    }

    /// Get all currently used registers that need to be saved
    pub fn get_used_registers(&self) -> Vec<String> {
        self.used_temp_registers.clone()
    }

    /// Reset temporary registers after a function completes
    pub fn reset_temp_registers(&mut self) {
        self.used_temp_registers.clear();

        // Restore original available registers
        self.available_temp_registers = vec![
            "t6".to_string(), "t5".to_string(), "t4".to_string(),
            "t3".to_string(), "t2".to_string(), "t1".to_string(), "t0".to_string()
        ];
    }

    pub fn declare_function(&mut self, name: &str, return_type: String, param_types: Vec<String>) {
        // Store the current function name
        self.current_function = Some(name.to_string());

        // Register the function with its return type
        self.register_function(name, return_type);
    }

    /// Initialize a global variable with a value
    pub fn initialize_global_variable(&mut self, name: String, value: i32) {
        let mut data_lines: Vec<String> = self.data_section.lines().map(String::from).collect();

        let mut i = 0;
        let var_declaration = format!("{}:", name);

        while i < data_lines.len() {
            if data_lines[i].trim() == var_declaration {
                // Found the variable declaration, update the next line
                if i + 1 < data_lines.len() {
                    data_lines[i + 1] = format!("    .word {}  # Global variable: {}", value, name);
                }
                break;
            }
            i += 1;
        }

        self.data_section = data_lines.join("\n") + "\n";
    }

    /// Initialize a global variable with a raw bit pattern (for floating-point)
    pub fn initialize_global_variable_raw(&mut self, name: String, value: u32) {
        // Update data section to include initialization value
        let mut data_lines: Vec<String> = self.data_section.lines()
                                               .map(String::from)
                                               .collect();

        let mut i = 0;
        let var_declaration = format!("{}:", name);

        while i < data_lines.len() {
            if data_lines[i].trim() == var_declaration {
                // Found the variable declaration, update the next line
                if i + 1 < data_lines.len() {
                    data_lines[i + 1] = format!("    .word 0x{:08x}  # Global variable: {}", value, name);
                }
                break;
            }
            i += 1;
        }

        self.data_section = data_lines.join("\n") + "\n";
    }

    /// Register a function with its return type
    pub fn register_function(&mut self, name: &str, return_type: String) {
        self.function_signatures.insert(name.to_string(), return_type);
    }

    /// Get the return type of a function
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

    /// Size of struct implementor
    pub fn get_struct_size(&self, struct_name: &str) -> Option<usize> {
        self.struct_definitions.get(struct_name).map(|def| def.total_size)
    }

    /// Adding the struct variable to the symbol table
    pub fn add_struct_variable(&mut self, var_name: String, struct_name: String) -> i32 {
        let struct_size = self.get_struct_size(&struct_name).unwrap_or(4);
        self.stack_offset -= struct_size as i32;
        let offset = self.stack_offset;
        self.symbols.last_mut().unwrap().insert(var_name, Symbol {
            location: StorageLocation::Stack(offset),
            size: struct_size,
            type_info: format!("struct {}", struct_name),
            dimensions: Vec::new(),
        });

        offset
    }
}
