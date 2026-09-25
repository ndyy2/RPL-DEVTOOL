#include "rplkit/runtime.h"

#include <iostream>
#include <string>
#include <vector>

#if !defined(_WIN32)
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#endif

#include "rplkit/module_manager.h"
#include "rplkit/system_services.h"

namespace rplkit {

namespace {
// Coba jalankan biner TUI Rust (target/release, lalu target/debug).
// Return: exit code anak bila berhasil dieksekusi, -1 bila tak ada biner
// (pemanggil jatuh ke menu legacy). Stdio diwariskan: TUI butuh tty asli.
int try_tui_binary(const std::string& repo_root) {
#if defined(_WIN32)
    (void)repo_root;
    return -1;
#else
    const std::string candidates[] = {
        repo_root + "/core/rust/target/release/rplkit",
        repo_root + "/core/rust/target/debug/rplkit",
    };
    std::string exe;
    for (const auto& c : candidates) {
        if (sys::file_exists(c) && ::access(c.c_str(), X_OK) == 0) {
            exe = c;
            break;
        }
    }
    if (exe.empty()) return -1;
    pid_t pid = ::fork();
    if (pid < 0) return -1;
    if (pid == 0) {
        ::execl(exe.c_str(), exe.c_str(), static_cast<char*>(nullptr));
        _exit(127);  // exec gagal
    }
    int status = 0;
    while (::waitpid(pid, &status, 0) < 0) {
    }
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    return 1;
#endif
}
}  // namespace

Runtime::Runtime(std::string repo_root) : repo_root_(std::move(repo_root)) {}

void Runtime::print_help(const char* prog) const {
    std::cout << "RPLKit Developer Tools (C++ runner, Rust runtime " << ModuleManager::runtime_version()
              << ")\n\n"
              << "Usage:\n"
              << "  " << prog << "                  interactive menu\n"
              << "  " << prog << " --list            list discovered tools\n"
              << "  " << prog << " --run <tool> [args...]\n"
              << "                                  run a tool\n"
              << "  " << prog << " --run <tool> [args...] --verbose\n"
              << "                                  also show which runner executed it\n"
              << "  " << prog << " --help            this help\n";
}

int Runtime::list_tools() const {
    ModuleManager mm(repo_root_);
    mm.discover();
    for (const auto& t : mm.tools()) {
        std::cout << t.name << " [" << t.runtime << "]  " << t.description << "\n";
        std::cout << "    entry: " << t.entry << "\n";
    }
    return 0;
}

int Runtime::interactive() const {
    ModuleManager mm(repo_root_);
    mm.discover();
    while (true) {
        std::cout << "\n+----------------------------------+\n"
                  << "|          RPL TOOLKIT             |\n"
                  << "|     Student Developer Tools      |\n"
                  << "|        (native C++ core)         |\n"
                  << "+----------------------------------+\n\n";
        const auto& tools = mm.tools();
        for (std::size_t i = 0; i < tools.size(); ++i) {
            std::cout << "[" << (i + 1) << "] " << tools[i].name << " (" << tools[i].runtime
                      << ")\n";
        }
        std::cout << "[0] Exit\n\nSelect tool > ";
        std::cout.flush();
        std::string choice;
        if (!std::getline(std::cin, choice)) {
            std::cout << "\n";
            return 0;
        }
        // trim
        std::size_t a = choice.find_first_not_of(" \t\r\n");
        if (a == std::string::npos) continue;
        std::size_t b = choice.find_last_not_of(" \t\r\n");
        choice = choice.substr(a, b - a + 1);
        if (choice == "0" || choice == "q" || choice == "exit") {
            std::cout << "Goodbye!\n";
            return 0;
        }
        // Numeric index or tool name.
        const ToolInfo* tool = nullptr;
        try {
            std::size_t idx = std::stoul(choice);
            if (idx >= 1 && idx <= tools.size()) tool = &tools[idx - 1];
        } catch (...) {
        }
        if (!tool) tool = mm.find(choice);
        if (!tool) {
            std::cout << "Unknown tool: " << choice << ". Try --list.\n";
            continue;
        }
        // Generic: one line of args (quote-aware split). Empty = interactive/none.
        std::cout << "arguments (empty = none/interactive) > ";
        std::cout.flush();
        std::string line;
        if (!std::getline(std::cin, line)) {
            std::cout << "\n";
            return 0;
        }
        mm.run(*tool, split_words(line), false);
    }
}

// Split shell sederhana: spasi pemisah, "..." dan '...' literal, \ escape.
std::vector<std::string> Runtime::split_words(const std::string& line) const {
    std::vector<std::string> out;
    std::string cur;
    bool in_s = false, in_d = false, esc = false, has = false;
    for (char c : line) {
        if (esc) {
            cur += c;
            esc = false;
            has = true;
        } else if (c == '\\' && !in_s) {
            esc = true;
        } else if (c == '\'' && !in_d) {
            in_s = !in_s;
            has = true;
        } else if (c == '"' && !in_s) {
            in_d = !in_d;
            has = true;
        } else if ((c == ' ' || c == '\t') && !in_s && !in_d) {
            if (has) {
                out.push_back(cur);
                cur.clear();
                has = false;
            }
        } else {
            cur += c;
            has = true;
        }
    }
    if (has) out.push_back(cur);
    return out;
}

int Runtime::run(int argc, char** argv) {
    std::string prog = (argc > 0 && argv[0]) ? argv[0] : "rplkit";
    if (argc < 2) {
        // Dua mode (mock): tanpa argumen → TUI Rust bila ada binernya,
        // else menu legacy (tanpa build).
        int tui = try_tui_binary(repo_root_);
        if (tui >= 0) return tui;
        sys::log("INFO", "rust TUI binary not found; using legacy menu");
        return interactive();
    }
    std::string cmd = argv[1];
    if (cmd == "--help" || cmd == "-h" || cmd == "help") {
        print_help(prog.c_str());
        return 0;
    }
    if (cmd == "--list" || cmd == "list") return list_tools();
    if (cmd == "--run" || cmd == "run") {
        if (argc < 3) {
            std::cerr << "usage: " << prog << " --run <tool> [args...]\n";
            return 2;
        }
        ModuleManager mm(repo_root_);
        mm.discover();
        const ToolInfo* tool = mm.find(argv[2]);
        if (!tool) {
            std::cerr << "unknown tool: " << argv[2] << "\n";
            return 1;
        }
        std::vector<std::string> args;
        for (int i = 3; i < argc; ++i) args.emplace_back(argv[i]);
        bool verbose = !args.empty() && args.back() == "--verbose";
        if (verbose) args.pop_back();
        return mm.run(*tool, args, verbose);
    }
    std::cerr << "unknown command: " << cmd << "\n";
    print_help(prog.c_str());
    return 2;
}

}  // namespace rplkit
