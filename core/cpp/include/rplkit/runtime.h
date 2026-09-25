#pragma once

#include <string>
#include <vector>

namespace rplkit {

// Application lifecycle: arg parsing, interactive menu, dispatch.
// Mirrors core/runtime.py so behaviour stays identical across cores.
class Runtime {
public:
    explicit Runtime(std::string repo_root);

    // Returns process exit code.
    int run(int argc, char** argv);

private:
    std::string repo_root_;

    int interactive() const;
    int list_tools() const;
    void print_help(const char* prog) const;
    std::vector<std::string> split_words(const std::string& line) const;
};

}  // namespace rplkit
