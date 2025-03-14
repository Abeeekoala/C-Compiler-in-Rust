#include "ast_node.hpp"

namespace ast {

NodeList::NodeList(NodePtr first_node) {
    nodes_.push_back(std::move(first_node));
}

void NodeList::PushBack(NodePtr item) {
    nodes_.push_back(std::move(item));
}

const std::vector<NodePtr>& NodeList::GetNodes() const {
    return nodes_;
}

void NodeList::EmitRISC(std::ostream& stream, Context& context) const {
    for (const auto& node : nodes_) {
        if (node) node->EmitRISC(stream, context);
    }
}

void NodeList::Print(std::ostream& stream) const {
    for (const auto& node : nodes_) {
        if (node) node->Print(stream);
    }
}

} // namespace ast
