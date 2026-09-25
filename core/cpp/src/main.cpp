// RPLKit native entry point. Resolves the repository root, then hands
// lifecycle control to rplkit::Runtime (see runtime.h).

#include <filesystem>
#include <string>

#include "rplkit/runtime.h"

namespace fs = std::filesystem;

static std::string find_repo_root(const char* argv0) {
    // Candidates: cwd, executable dir, and up to 4 parents of each.
    // A directory counts as repo root when it contains tools/, config/,
    // main.py, or pcc.conf.json.
    auto is_root = [](const fs::path& p) -> bool {
        try {
            return fs::is_directory(p / "tools") || fs::is_directory(p / "config") ||
                   fs::exists(p / "main.py") || fs::exists(p / "pcc.conf.json");
        } catch (...) {
            return false;
        }
    };
    fs::path starts[2];
    int n = 0;
    try {
        starts[n++] = fs::current_path();
    } catch (...) {
    }
    try {
        fs::path exe = fs::absolute(argv0 ? argv0 : ".");
        if (!fs::is_directory(exe)) exe = exe.parent_path();
        starts[n++] = exe;
    } catch (...) {
    }
    for (int i = 0; i < n; ++i) {
        fs::path p = starts[i];
        for (int up = 0; up < 5; ++up) {
            if (is_root(p)) return p.string();
            if (!p.has_parent_path()) break;
            p = p.parent_path();
        }
    }
    try {
        return fs::current_path().string();
    } catch (...) {
        return ".";
    }
}

int main(int argc, char** argv) {
    std::string root = find_repo_root(argc > 0 ? argv[0] : nullptr);
    rplkit::Runtime rt(root);
    return rt.run(argc, argv);
}
