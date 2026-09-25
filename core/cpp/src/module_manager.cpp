#include "rplkit/module_manager.h"

#include <filesystem>
#include <iostream>
#include <sstream>

#include "rplkit/native_tools.h"
#include "rplkit/system_services.h"

namespace rplkit {
namespace fs = std::filesystem;

ModuleManager::ModuleManager(std::string repo_root) : repo_root_(std::move(repo_root)) {}

std::vector<ToolInfo> ModuleManager::native_builtins() {
    return {
        {"calc", "native", "calc", "Safe arithmetic evaluator (C++ core)"},
        {"b64enc", "native", "b64enc", "Base64 encode stdin/arg (C++ core)"},
        {"b64dec", "native", "b64dec", "Base64 decode stdin/arg (C++ core)"},
        {"hexenc", "native", "hexenc", "Hex encode (C++ core)"},
        {"hexdec", "native", "hexdec", "Hex decode (C++ core)"},
        {"uuid", "native", "uuid", "Random UUID v4 (C++ core)"},
        {"timestamp", "native", "timestamp", "Unix + ISO-8601 UTC time (C++ core)"},
        {"file-info", "native", "file-info", "File size / line count (C++ core)"},
    };
}

static std::string join_args(const std::vector<std::string>& args) {
    std::string cmd;
    for (const auto& a : args) {
        if (!cmd.empty()) cmd += " ";
        cmd += sys::shell_quote(a);
    }
    return cmd;
}

static std::string describe_script(const std::string& script_path, const std::string& fallback) {
    // Sibling manifest: <tool>.json next to the script, {"description": "..."}.
    std::string manifest = script_path.substr(0, script_path.find_last_of('.')) + ".json";
    std::string text = sys::read_file(manifest);
    if (!text.empty()) {
        std::string d = sys::json_string_field(text, "description");
        if (!d.empty()) return d;
    }
    // Fallback: first docstring/comment line of the script.
    std::string src = sys::read_file(script_path);
    std::istringstream in(src);
    std::string line;
    for (int i = 0; i < 6 && std::getline(in, line); ++i) {
        if (line.find("\"\"\"") != std::string::npos || line.find("///") != std::string::npos ||
            line.find("//") != std::string::npos) {
            std::string t;
            for (char c : line) {
                if (c != '"' && c != '/' && c != '#' && c != '\'') t += c;
            }
            std::size_t a = t.find_first_not_of(" \t");
            if (a != std::string::npos) {
                t = t.substr(a);
                if (!t.empty()) return t;
            }
        }
    }
    return fallback;
}

void ModuleManager::add_script_tools(const std::string& dir, const std::string& runtime) {
    std::string ext = (runtime == "python") ? ".py" : ".ts";
    try {
        if (!fs::is_directory(dir)) return;
        for (const auto& e : fs::directory_iterator(dir)) {
            if (!e.is_regular_file()) continue;
            if (e.path().extension() != ext) continue;
            std::string stem = e.path().stem().string();
            if (stem == "__init__") continue;
            ToolInfo t{stem, runtime, e.path().string(),
                       describe_script(e.path().string(), runtime + " tool: " + stem)};
            tools_.push_back(std::move(t));
        }
    } catch (...) {
    }
}

void ModuleManager::add_legacy_python_tools() {
    // Pre-refactor layout: flat tools/*.py (calculator.py, ...).
    add_script_tools((fs::path(repo_root_) / "tools").string(), "python");
}

void ModuleManager::discover() {
    tools_.clear();
    for (const auto& b : native_builtins()) tools_.push_back(b);
    add_script_tools((fs::path(repo_root_) / "tools" / "python").string(), "python");
    add_script_tools((fs::path(repo_root_) / "tools" / "typescript").string(), "typescript");
    // Legacy flat layout, skipped when the new layout already provided the name.
    std::size_t before = tools_.size();
    add_legacy_python_tools();
    // De-duplicate by name, keeping the first (new layout wins).
    std::vector<ToolInfo> uniq;
    for (auto& t : tools_) {
        bool seen = false;
        for (auto& u : uniq) {
            if (u.name == t.name) {
                seen = true;
                break;
            }
        }
        if (!seen) uniq.push_back(t);
    }
    (void)before;
    tools_ = std::move(uniq);
}

const ToolInfo* ModuleManager::find(const std::string& name) const {
    for (const auto& t : tools_) {
        if (t.name == name) return &t;
    }
    return nullptr;
}

int ModuleManager::run_native(const ToolInfo& tool, const std::vector<std::string>& args) const {
    try {
        const std::string& id = tool.entry;
        std::string arg0 = args.empty() ? "" : args[0];
        if (id == "calc") {
            if (args.empty()) {
                std::cerr << "usage: rplkit --run calc \"<expression>\"\n";
                return 2;
            }
            std::cout << native::calc(arg0) << "\n";
            return 0;
        }
        if (id == "b64enc" || id == "hexenc") {
            std::string input = arg0;
            if (args.empty()) std::getline(std::cin, input);
            std::cout << (id == "b64enc" ? native::b64encode(input) : native::hex_encode(input))
                      << "\n";
            return 0;
        }
        if (id == "b64dec" || id == "hexdec") {
            std::string input = arg0;
            if (args.empty()) std::getline(std::cin, input);
            std::cout << (id == "b64dec" ? native::b64decode(input) : native::hex_decode(input))
                      << "\n";
            return 0;
        }
        if (id == "uuid") {
            int n = 1;
            if (!args.empty()) {
                try {
                    n = std::stoi(args[0]);
                } catch (...) {
                    n = 1;
                }
            }
            if (n < 1) n = 1;
            if (n > 100) n = 100;
            for (int i = 0; i < n; ++i) std::cout << native::uuid4() << "\n";
            return 0;
        }
        if (id == "timestamp") {
            std::cout << native::unix_now() << "\n" << native::iso_now() << "\n";
            return 0;
        }
        if (id == "file-info") {
            if (args.empty()) {
                std::cerr << "usage: rplkit --run file-info <path>\n";
                return 2;
            }
            std::cout << "exists: " << (sys::file_exists(arg0) ? "yes" : "no") << "\n";
            std::cout << "size: " << sys::file_size(arg0) << " bytes\n";
            std::cout << "lines: " << sys::count_lines(arg0) << "\n";
            return 0;
        }
        std::cerr << "unknown native tool: " << id << "\n";
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "error: " << e.what() << "\n";
        return 1;
    }
}

int ModuleManager::run_python(const ToolInfo& tool, const std::vector<std::string>& args) const {
    std::string cmd = "python3 " + sys::shell_quote(tool.entry);
    std::string tail = join_args(args);
    if (!tail.empty()) cmd += " " + tail;
    ProcessResult r = sys::run_process(cmd);
    std::cout << r.output;
    return r.exit_code;
}

int ModuleManager::run_typescript(const ToolInfo& tool, const std::vector<std::string>& args) const {
    // node >= 22 strips types natively; older node prints a clear error.
    std::string cmd = "node " + sys::shell_quote(tool.entry);
    std::string tail = join_args(args);
    if (!tail.empty()) cmd += " " + tail;
    ProcessResult r = sys::run_process(cmd);
    std::cout << r.output;
    if (r.exit_code != 0) {
        sys::log("WARN", "typescript tool failed; is node installed? (" + tool.entry + ")");
    }
    return r.exit_code;
}

int ModuleManager::run(const ToolInfo& tool, const std::vector<std::string>& args) const {
    if (tool.runtime == "native") return run_native(tool, args);
    if (tool.runtime == "python") return run_python(tool, args);
    if (tool.runtime == "typescript") return run_typescript(tool, args);
    std::cerr << "unsupported runtime: " << tool.runtime << "\n";
    return 1;
}

}  // namespace rplkit
