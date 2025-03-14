#pragma once

#include <iostream>
#include <memory>
#include <vector>

#include "ast_context.hpp"

namespace ast {

class Node
{
public:
    virtual ~Node() {}
    virtual void EmitRISC(std::ostream& stream, Context& context) const = 0;
    virtual void Print(std::ostream& stream) const = 0;
};

using NodePtr = std::unique_ptr<const Node>;

class NodeList : public Node {
private:
    std::vector<NodePtr> nodes_;

public:
    NodeList(NodePtr first_node);
    void PushBack(NodePtr item);
    const std::vector<NodePtr>& GetNodes() const;

    void EmitRISC(std::ostream& stream, Context& context) const override;
    void Print(std::ostream& stream) const override;
};

} // namespace ast
