#pragma once

#include <string>
#include <vector>

namespace rplkit {

// One entry in the System Module registry.
struct ToolInfo {
    std::string name;         // e.g. "calculator"
    std::string runtime;      // "native" | "python" | "typescript"
    std::string entry;        // native: builtin id; else path to script
    std::string description;
};

class ModuleManager {
public:
    explicit ModuleManager(std::string repo_root);

    // Re-scan tools/{python,typescript} + legacy tools/*.py + native builtins.
    // Never throws; missing dirs simply yield fewer tools.
    void discover();

    const std::vector<ToolInfo>& tools() const { return tools_; }
    const ToolInfo* find(const std::string& name) const;
    const std::string& repo_root() const { return repo_root_; }

    // Dispatch helpers. Return process exit code (0 ok).
    int run_native(const ToolInfo& tool, const std::vector<std::string>& args) const;
    int run_python(const ToolInfo& tool, const std::vector<std::string>& args) const;
    int run_typescript(const ToolInfo& tool, const std::vector<std::string>& args) const;
    int run(const ToolInfo& tool, const std::vector<std::string>& args) const;

    // Native builtins always available, even with no tools/ on disk.
    static std::vector<ToolInfo> native_builtins();

private:
    std::string repo_root_;
    std::vector<ToolInfo> tools_;

    void add_script_tools(const std::string& dir, const std::string& runtime);
    void add_legacy_python_tools();
};

}  // namespace rplkit
