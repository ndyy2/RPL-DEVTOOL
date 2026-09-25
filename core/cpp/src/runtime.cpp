#include "rplkit/runtime.h"

#include <iostream>
#include <string>
#include <vector>

#include "rplkit/module_manager.h"
#include "rplkit/system_services.h"

namespace rplkit {

Runtime::Runtime(std::string repo_root) : repo_root_(std::move(repo_root)) {}

void Runtime::print_help(const char* prog) const {
    std::cout << "RPLKit Developer Tools (native C++ core)\n\n"
              << "Usage:\n"
              << "  " << prog << "                  interactive menu\n"
              << "  " << prog << " --list            list discovered tools\n"
              << "  " << prog << " --run <tool> [args...]\n"
              << "                                  run a tool\n"
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
        if (tool->runtime == "native") {
            // Native interactive shortcuts: prompt for the single arg.
            std::vector<std::string> args;
            if (tool->entry == "calc" || tool->entry == "b64enc" || tool->entry == "b64dec" ||
                tool->entry == "hexenc" || tool->entry == "hexdec" ||
                tool->entry == "file-info") {
                std::cout << "argument (empty = cancel) > ";
                std::cout.flush();
                std::string arg;
                if (!std::getline(std::cin, arg)) {
                    std::cout << "\n";
                    return 0;
                }
                if (arg.empty()) continue;
                args.push_back(arg);
            }
            mm.run(*tool, args);
        } else {
            mm.run(*tool, {});
        }
    }
}

int Runtime::run(int argc, char** argv) {
    std::string prog = (argc > 0 && argv[0]) ? argv[0] : "rplkit";
    if (argc < 2) return interactive();
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
        return mm.run(*tool, args);
    }
    std::cerr << "unknown command: " << cmd << "\n";
    print_help(prog.c_str());
    return 2;
}

}  // namespace rplkit
