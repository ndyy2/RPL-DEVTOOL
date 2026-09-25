#pragma once

#include <string>
#include <vector>

namespace rplkit {

// Satu entri registry. Diisi DARI Rust (rplkit_list_tools) — C++ tidak
// lagi tahu cara discovery maupun cara menjalankan tool.
struct ToolInfo {
    std::string name;         // e.g. "calculator"
    std::string runtime;      // "native" | "python" | "typescript"
    std::string entry;        // native: op id; else absolute script path
    std::string description;
};

class ModuleManager {
public:
    explicit ModuleManager(std::string repo_root);

    // Tanya registry Rust. Never throws; gagal = daftar kosong.
    void discover();

    const std::vector<ToolInfo>& tools() const { return tools_; }
    const ToolInfo* find(const std::string& name) const;
    const std::string& repo_root() const { return repo_root_; }

    // Jalankan via chain Rust. verbose=true → cetak executed_by ke stderr.
    // Return process exit code (0 ok).
    int run(const ToolInfo& tool, const std::vector<std::string>& args,
            bool verbose = false) const;

    static const char* runtime_version();

private:
    std::string repo_root_;
    std::vector<ToolInfo> tools_;
};

}  // namespace rplkit
