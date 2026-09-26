#include "rplkit/module_manager.h"

#include <cctype>
#include <iostream>

#include "rplkit/runtime_ffi.h"
#include "rplkit/system_services.h"

namespace rplkit {

// RAII untuk string milik Rust.
class FfiString {
public:
    explicit FfiString(char* p) : p_(p) {}
    ~FfiString() {
        if (p_) rplkit_str_free(p_);
    }
    FfiString(const FfiString&) = delete;
    FfiString& operator=(const FfiString&) = delete;
    const char* get() const { return p_ ? p_ : ""; }
    bool null() const { return p_ == nullptr; }

private:
    char* p_;
};

ModuleManager::ModuleManager(std::string repo_root) : repo_root_(std::move(repo_root)) {}

// Ekstrak objek-objek {...} level-atas dari JSON array (string-aware).
static std::vector<std::string> split_objects(const std::string& json) {
    std::vector<std::string> objs;
    std::size_t i = 0;
    while (i < json.size()) {
        if (json[i] != '{') {
            ++i;
            continue;
        }
        int depth = 0;
        bool in_str = false, esc = false;
        std::size_t start = i;
        for (; i < json.size(); ++i) {
            char c = json[i];
            if (in_str) {
                if (esc) {
                    esc = false;
                } else if (c == '\\') {
                    esc = true;
                } else if (c == '"') {
                    in_str = false;
                }
            } else if (c == '"') {
                in_str = true;
            } else if (c == '{') {
                ++depth;
            } else if (c == '}') {
                if (--depth == 0) {
                    ++i;
                    break;
                }
            }
        }
        objs.push_back(json.substr(start, i - start));
    }
    return objs;
}

void ModuleManager::discover() {
    tools_.clear();
    try {
        FfiString res(rplkit_list_tools(repo_root_.c_str()));
        if (res.null()) {
            sys::log("ERROR", "rplkit_list_tools returned NULL");
            return;
        }
        for (const auto& obj : split_objects(res.get())) {
            ToolInfo t{sys::json_string_field(obj, "name"), sys::json_string_field(obj, "runtime"),
                       sys::json_string_field(obj, "entry"),
                       sys::json_string_field(obj, "description")};
            if (!t.name.empty()) tools_.push_back(std::move(t));
        }
    } catch (...) {
        tools_.clear();
    }
}

const ToolInfo* ModuleManager::find(const std::string& name) const {
    for (const auto& t : tools_) {
        if (t.name == name) return &t;
    }
    return nullptr;
}

static std::string to_json_array(const std::vector<std::string>& args) {
    std::string out = "[";
    for (std::size_t i = 0; i < args.size(); ++i) {
        if (i) out += ",";
        out += "\"";
        for (char c : args[i]) {
            switch (c) {
                case '"': out += "\\\""; break;
                case '\\': out += "\\\\"; break;
                case '\n': out += "\\n"; break;
                case '\r': out += "\\r"; break;
                case '\t': out += "\\t"; break;
                default: out += c; break;
            }
        }
        out += "\"";
    }
    out += "]";
    return out;
}

static int parse_code(const std::string& json) {
    // {"code":N,...} — N integer (boleh negatif).
    std::size_t pos = json.find("\"code\"");
    if (pos == std::string::npos) return 1;
    pos = json.find(':', pos);
    if (pos == std::string::npos) return 1;
    ++pos;
    while (pos < json.size() && std::isspace(static_cast<unsigned char>(json[pos]))) ++pos;
    try {
        return std::stoi(json.substr(pos));
    } catch (...) {
        return 1;
    }
}

// Angka setelah "key": ... ; -1 bila tak ketemu.
static long parse_long_field(const std::string& json, const char* key) {
    std::string q = std::string("\"") + key + "\"";
    std::size_t pos = json.find(q);
    if (pos == std::string::npos) return -1;
    pos = json.find(':', pos);
    if (pos == std::string::npos) return -1;
    ++pos;
    while (pos < json.size() && std::isspace(static_cast<unsigned char>(json[pos]))) ++pos;
    try {
        return std::stol(json.substr(pos));
    } catch (...) {
        return -1;
    }
}

int ModuleManager::run(const ToolInfo& tool, const std::vector<std::string>& args,
                       bool verbose) const {
    try {
        std::string aj = to_json_array(args);
        FfiString res(rplkit_run_tool(repo_root_.c_str(), tool.name.c_str(), aj.c_str()));
        if (res.null()) {
            std::cerr << "error: runtime returned NULL\n";
            return 1;
        }
        std::string text = res.get();
        if (verbose) {
            std::string by = sys::json_string_field(text, "executed_by");
            if (!by.empty()) std::cerr << "executed_by: " << by << "\n";
            long dms = parse_long_field(text, "discover_ms");
            if (dms >= 0) std::cerr << "discover: " << dms << "ms\n";
            for (const auto& obj : split_objects(text)) {
                std::string step = sys::json_string_field(obj, "step");
                if (step.empty()) continue;  // bukan entri attempts
                long ms = parse_long_field(obj, "ms");
                bool ok = obj.find("\"ok\":true") != std::string::npos;
                std::cerr << "  step " << step << ": " << ms << "ms " << (ok ? "ok" : "skip/fail")
                          << "\n";
            }
        }
        return parse_code(text);
    } catch (const std::exception& e) {
        std::cerr << "error: " << e.what() << "\n";
        return 1;
    } catch (...) {
        std::cerr << "error: unknown failure in runtime\n";
        return 1;
    }
}

const char* ModuleManager::runtime_version() {
    try {
        return rplkit_version();
    } catch (...) {
        return "unknown";
    }
}

}  // namespace rplkit
