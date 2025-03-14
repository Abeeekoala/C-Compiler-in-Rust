#pragma once
#include <unordered_map>
#include <string>

namespace ast {

class Context {
private:
    std::unordered_map<std::string, int> variable_offsets_;
    int stack_offset_ = 0; // Tracks current stack position

public:
    int AllocateVariable(const std::string& name) {
        stack_offset_ -= 4; // Reserve 4 bytes for the variable
        variable_offsets_[name] = stack_offset_;
        return stack_offset_;
    }

    int GetVariableOffset(const std::string& name) const {
        auto it = variable_offsets_.find(name);
        if (it != variable_offsets_.end()) {
            return it->second;
        }
        throw ("Undefined variable: " + name);
    }
};

} // namespace ast
