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

/// Manages the compilation context
#[derive(Debug)]
pub struct CodeGenContext {
    /// Current function name
    pub current_function: Option<String>,
    /// Symbol table mapping variable names to their storage locations
    pub symbols: HashMap<String, Symbol>,
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
    /// Next temporary register counter when we run out of predefined registers
    next_temp_reg: usize,

    break_labels: Vec<String>,
}

impl CodeGenContext {
    /// Create a new code generation context
    pub fn new() -> Self {
        // Initialize with RISC-V calling convention registers
        let temp_regs = vec![
            "t6", "t5", "t4", "t3", "t2", "t1", "t0",
        ];

        CodeGenContext {
            current_function: None,
            symbols: HashMap::new(),
            stack_offset: 0,
            label_counter: 0,
            output: String::new(),
            data_section: String::new(),
            in_function: false,
            used_temp_registers: Vec::new(),
            available_temp_registers: temp_regs.iter().map(|&s| s.to_string()).collect(),
            next_temp_reg: 0,
            break_labels: Vec::new(),
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

    /// Generate a unique label
    pub fn generate_label(&mut self, prefix: &str) -> String {
        let label = format!(".{}{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Allocate a register for temporary use
    pub fn allocate_register(&mut self) -> Option<String> {
        if let Some(reg) = self.available_temp_registers.pop() {
            self.used_temp_registers.push(reg.clone());
            Some(reg)
        } else {
            // If we're out of predefined registers, generate a new one
            let reg = format!("t{}", self.next_temp_reg);
            self.next_temp_reg += 1;
            self.used_temp_registers.push(reg.clone());
            Some(reg)
        }
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

    /// Allocate space on the stack for a variable
    pub fn allocate_stack_space(&mut self, size: usize) -> i32 {
        // Align to 4 bytes
        let aligned_size = (size + 3) & !3;
        self.stack_offset -= aligned_size as i32;
        self.stack_offset
    }

    /// Add a symbol to the symbol table
    pub fn add_symbol(&mut self, name: &str, symbol: Symbol) {
        self.symbols.insert(name.to_string(), symbol);
    }

    /// Look up a symbol in the symbol table
    pub fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Get a variable's offset and type from the symbol table
    pub fn get_variable(&self, name: &str) -> Option<(i32, String)> {
        self.symbols.get(name).map(|symbol| {
            if let StorageLocation::Stack(offset) = symbol.location {
                (offset, symbol.type_info.clone())
            } else {
                panic!("Variable not on stack")
            }
        })
    }

    /// Generate the function prologue with support for recursion
    pub fn generate_function_prologue(&mut self) {
        self.in_function = true;

        // Save the old stack frame
        self.emit("    addi sp, sp, -32");      // Make space for saved registers
        self.emit("    sw ra, 28(sp)");         // Save return address
        self.emit("    sw fp, 24(sp)");         // Save old frame pointer
        self.emit("    addi fp, sp, 32");       // Set up new frame pointer

        // Save callee-saved registers we'll use
        self.emit("    sw s1, 20(sp)");
        self.emit("    sw s2, 16(sp)");

        // Reserve space for local variables (aligned to 16 bytes)
        self.emit("    addi sp, sp, -32");      // Initial space for locals

        self.stack_offset = -32;                 // Track stack allocations from here
    }

    /// Generate the function epilogue with proper cleanup
    pub fn generate_function_epilogue(&mut self) {
        // Deallocate local variables
        self.emit("    mv sp, fp");             // Restore stack pointer
        self.emit("    addi sp, sp, -32");      // Point to saved registers

        // Restore saved registers
        self.emit("    lw s2, 16(sp)");
        self.emit("    lw s1, 20(sp)");
        self.emit("    lw fp, 24(sp)");
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
        if (!self.data_section.is_empty()) {
            full_assembly.push_str(".data\n");
            full_assembly.push_str(&self.data_section);
            full_assembly.push_str("\n");
        }

        // Add text section
        full_assembly.push_str(".text\n");
        full_assembly.push_str(&self.output);

        full_assembly
    }

    /// Add a variable to the symbol table
    pub fn add_variable(&mut self, name: String, type_name: String) -> i32 {
        if self.in_function {
            // Local variable - allocate on stack
            self.stack_offset -= 4;
            let offset = self.stack_offset;

            self.symbols.insert(name, Symbol {
                location: StorageLocation::Stack(offset),
                size: 4,
                type_info: type_name,
                dimensions: Vec::new(),
            });

            offset
        } else {
            // Global variable - add to data section
            let label = name.clone();
            self.emit_data(&format!("{}:", label));
            self.emit_data(&format!("    .word 0  # Global variable: {}", name));

            self.symbols.insert(name, Symbol {
                location: StorageLocation::Global(label),
                size: 4,
                type_info: type_name,
                dimensions: Vec::new(),
            });

            0 // Return value doesn't matter for globals
        }
    }

    /// Get a register for temporary use
    pub fn get_register(&mut self) -> String {
        // Always allocate a new register, don't reuse existing ones
        self.allocate_register().unwrap_or_else(|| {
            // This is a fallback in case allocate_register fails (which shouldn't happen)
            let reg = format!("t{}", self.next_temp_reg);
            self.next_temp_reg += 1;
            self.used_temp_registers.push(reg.clone());
            reg
        })
    }

    /// Get all currently used registers that need to be saved
    pub fn get_used_registers(&self) -> Vec<String> {
        self.used_temp_registers.clone()
    }

    /// Reset temporary registers after a function completes
    pub fn reset_temp_registers(&mut self) {
        self.used_temp_registers.clear();
        self.next_temp_reg = 0;

        // Restore original available registers
        self.available_temp_registers = vec![
            "t6".to_string(), "t5".to_string(), "t4".to_string(),
            "t3".to_string(), "t2".to_string(), "t1".to_string(), "t0".to_string()
        ];
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
            self.symbols.insert(name.clone(), Symbol {
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
            self.symbols.insert(name, Symbol {
                location: StorageLocation::Global(label),
                size: total_size,
                type_info: array_type,
                dimensions: dimensions,
            });

            0 // Return value doesn't matter for globals
        }
    }

    pub fn enter_scope(&mut self) {
        //  - will be needed for nested scopes
    }

    pub fn exit_scope(&mut self) {
        //  - will be needed for nested scopes
    }

    pub fn declare_function(&mut self, name: &str, param_types: Vec<String>) {
        // Just storinh the current function name
        self.current_function = Some(name.to_string());
    }
}
