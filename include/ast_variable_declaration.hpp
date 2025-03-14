#pragma once

#include "ast_node.hpp"

namespace ast {

class VariableDeclaration : public Node {
private:
    std::string name_;
    NodePtr initializer_;

public:
    VariableDeclaration(std::string name, NodePtr initializer)
        : name_(std::move(name)), initializer_(std::move(initializer)) {}

    virtual void EmitRISC(std::ostream& stream, Context& context) const override {
        int offset = context.AllocateVariable(name_);
        if (initializer_) {
            initializer_->EmitRISC(stream, context);
            stream << "  sw a0, " << offset << "(sp) # Store variable " << name_ << "\n";
        }
    }

    virtual void Print(std::ostream& stream) const override {
        stream << "VariableDeclaration(" << name_;
        if (initializer_) {
            stream << " = ";
            initializer_->Print(stream);
        }
        stream << ")";
    }
};

class Assignment : public Node {
private:
    std::string name_;
    NodePtr expression_;

public:
    Assignment(std::string name, NodePtr expression)
        : name_(std::move(name)), expression_(std::move(expression)) {}

    void EmitRISC(std::ostream& stream, Context& context) const override {
        expression_->EmitRISC(stream, context);
        int offset = context.GetVariableOffset(name_);
        stream << "  sw a0, " << offset << "(sp) # Store " << name_ << "\n";
    }

    void Print(std::ostream& stream) const override {
        stream << name_ << " = ";
        expression_->Print(stream);
        stream << ";";
    }
};

} // namespace ast
