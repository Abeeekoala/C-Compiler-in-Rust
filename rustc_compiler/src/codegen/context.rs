use std::collections::HashMap;

/// Types of locations where variables can be stored
#[derive(Debug, Clone)]
pub enum StorageLocation {
    /// Register allocation
    Register(String),
    /// Stack allocation with offset from frame pointer
    Stack(i32),
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
}

/// Manages the compilation context
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
    /// Temporary registers currently in use
    pub used_temp_registers: Vec<String>,
    /// Available temporary registers
    pub available_temp_registers: Vec<String>,
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
            used_temp_registers: Vec::new(),
            available_temp_registers: temp_regs.iter().map(|&s| s.to_string()).collect(),
        }
    }

    /// Add a line of assembly to the output
    pub fn emit(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
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
            None  // No registers available
        }
    }

    /// Free a previously allocated register
    pub fn free_register(&mut self, reg: &str) {
        if let Some(pos) = self.used_temp_registers.iter().position(|r| r == reg) {
            self.used_temp_registers.remove(pos);
            self.available_temp_registers.push(reg.to_string());
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
}
