// src/analyzer.rs
use crate::ast::{AstNode, TypeSpecifier};

pub fn compute_local_stack_size(ast: &AstNode) -> i32 {
    let mut size = 0;
    match ast {
        AstNode::Declaration { type_spec, declarator: _, initializer: _ } => {
            size += get_type_size(type_spec);
        },
        AstNode::NodeList(nodes) => {
            for node in nodes {
                size += compute_local_stack_size(node);
            }
        },
        _ => {}
    }
    size
}

fn get_type_size(type_spec: &TypeSpecifier) -> i32 {
    // Simple mapping (aligned to 4 bytes)
    match type_spec {
        TypeSpecifier::Int => 4,
        TypeSpecifier::Float => 4,
        TypeSpecifier::Double => 8,
        TypeSpecifier::Char => 4,
        _ => 4,
    }
}
